use axum::{extract::{Path, Query, State}, Json};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    domain::rental::{
        RentalBookingResponse, CreateRentalRequest,
        ValidatePickupRequest, ValidateReturnRequest, CancelRentalRequest,
        UpdateRentalStatusRequest, RentalStatus,
    },
    error::AppError,
    repositories::rental_repo,
    AppState,
};

use crate::middleware::auth::{AuthUser, AuthCustomer, AuthSeller};
use shared::utils::validation;

#[derive(Debug, Serialize, ToSchema)]
pub struct MessageResponse {
    pub message: String,
}

#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct RentalFilter {
    pub status: Option<String>,
}

// Create rental booking baru (customer)
#[utoipa::path(
    post,
    path = "/api/rentals/bookings",
    tag = "Rental Bookings",
    security(("bearer_auth" = [])),
    request_body = CreateRentalRequest,
    responses(
        (status = 201, description = "Rental booking created", body = RentalBookingResponse),
        (status = 400, description = "Input tidak valid"),
        (status = 409, description = "Vehicle tidak available"),
    )
)]
pub async fn create_rental_booking(
    auth: AuthCustomer,
    State(state): State<AppState>,
    Json(payload): Json<CreateRentalRequest>,
) -> Result<Json<RentalBookingResponse>, AppError> {
    tracing::info!(
        "Customer {} ({}) creating rental for vehicle {}",
        auth.user_id,
        auth.email,
        payload.vehicle_id
    );

    // Validasi input
    validate_create_rental(&payload)?;

    // Check vehicle exists dan ambil seller_id + price dari vehicle-service API
    let url = format!("{}/vehicles/{}/rental-info",
        state.config.vehicle_service_url,
        payload.vehicle_id
    );

    #[derive(serde::Deserialize)]
    struct VehicleRentalInfo {
        id: i32,
        seller_id: i32,
        price_per_day: f64,
        is_available: bool,
    }

    let response = state.http_client
        .get(&url)
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| AppError::database_error(format!("Gagal menghubungi vehicle-service: {}", e)))?;

    if !response.status().is_success() {
        return Err(AppError::not_found("Vehicle tidak tersedia untuk rental"));
    }

    let vehicle_info: VehicleRentalInfo = response
        .json()
        .await
        .map_err(|e| AppError::database_error(format!("Gagal parse response: {}", e)))?;

    if !vehicle_info.is_available {
        return Err(AppError::Conflict("Vehicle tidak tersedia untuk rental".to_string()));
    }

    let (vehicle_id, seller_id, price_per_day) = (vehicle_info.id, vehicle_info.seller_id, vehicle_info.price_per_day);

    // Check availability
    let is_available = rental_repo::check_vehicle_availability(
        &state.db,
        vehicle_id,
        payload.pickup_date,
        payload.return_date,
    ).await?;

    if !is_available {
        return Err(AppError::conflict("Vehicle sudah dibooking di tanggal tersebut"));
    }

    // Create rental booking
    let rental = rental_repo::create_rental(
        &state.db,
        auth.user_id,
        seller_id,
        price_per_day,
        &payload,
    ).await?;

    tracing::info!("Rental booking {} created", rental.order_id);

    Ok(Json(RentalBookingResponse::from(rental)))
}

// Get rental booking by ID
#[utoipa::path(
    get,
    path = "/api/rentals/bookings/{id}",
    tag = "Rental Bookings",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "Rental booking ID")),
    responses(
        (status = 200, description = "Rental booking detail", body = RentalBookingResponse),
        (status = 404, description = "Rental booking tidak ditemukan"),
    )
)]
pub async fn get_rental_booking(
    auth: AuthUser,
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<RentalBookingResponse>, AppError> {
    let rental = rental_repo::find_rental_by_id(&state.db, id)
        .await?
        .ok_or_else(|| AppError::not_found("Rental booking tidak ditemukan"))?;

    // Check authorization (customer or seller)
    if rental.customer_id != auth.user_id && rental.seller_id != auth.user_id {
        return Err(AppError::forbidden("Anda tidak memiliki akses ke booking ini"));
    }

    Ok(Json(RentalBookingResponse::from(rental)))
}

// List my rental bookings (customer)
#[utoipa::path(
    get,
    path = "/api/rentals/bookings/my",
    tag = "Rental Bookings",
    security(("bearer_auth" = [])),
    params(RentalFilter),
    responses(
        (status = 200, description = "List rental bookings", body = Vec<RentalBookingResponse>),
    )
)]
pub async fn get_customer_rental_bookings(
    auth: AuthCustomer,
    Query(filter): Query<RentalFilter>,
    State(state): State<AppState>,
) -> Result<Json<Vec<RentalBookingResponse>>, AppError> {
    let rentals = rental_repo::find_rentals_by_customer(
        &state.db,
        auth.user_id,
        filter.status,
    ).await?;

    let response: Vec<RentalBookingResponse> = rentals
        .into_iter()
        .map(RentalBookingResponse::from)
        .collect();

    Ok(Json(response))
}

// List seller rental bookings (seller)
#[utoipa::path(
    get,
    path = "/api/rentals/bookings/seller",
    tag = "Rental Bookings",
    security(("bearer_auth" = [])),
    params(RentalFilter),
    responses(
        (status = 200, description = "List rental bookings", body = Vec<RentalBookingResponse>),
    )
)]
pub async fn get_seller_rental_bookings(
    auth: AuthSeller,
    Query(filter): Query<RentalFilter>,
    State(state): State<AppState>,
) -> Result<Json<Vec<RentalBookingResponse>>, AppError> {
    let rentals = rental_repo::find_rentals_by_seller(
        &state.db,
        auth.user_id,
        filter.status,
    ).await?;

    let response: Vec<RentalBookingResponse> = rentals
        .into_iter()
        .map(RentalBookingResponse::from)
        .collect();

    Ok(Json(response))
}

// Validate pickup (seller)
#[utoipa::path(
    put,
    path = "/api/rentals/bookings/{id}/pickup",
    tag = "Rental Bookings",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "Rental booking ID")),
    request_body = ValidatePickupRequest,
    responses(
        (status = 200, description = "Pickup validated", body = RentalBookingResponse),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found"),
    )
)]
pub async fn validate_pickup(
    auth: AuthSeller,
    Path(id): Path<i32>,
    State(state): State<AppState>,
    Json(payload): Json<ValidatePickupRequest>,
) -> Result<Json<RentalBookingResponse>, AppError> {
    let rental = rental_repo::find_rental_by_id(&state.db, id)
        .await?
        .ok_or_else(|| AppError::not_found("Rental booking tidak ditemukan"))?;

    if rental.seller_id != auth.user_id {
        return Err(AppError::forbidden("Anda bukan seller dari vehicle ini"));
    }

    if rental.status != "paid" && rental.status != "akan_datang" {
        return Err(AppError::bad_request("Status rental tidak valid untuk pickup"));
    }

    let updated = rental_repo::validate_pickup(&state.db, id, &payload.ktp_photo).await?;

    tracing::info!("Rental {} pickup validated by seller {}", id, auth.user_id);

    Ok(Json(RentalBookingResponse::from(updated)))
}

// Validate return (seller)
#[utoipa::path(
    put,
    path = "/api/rentals/bookings/{id}/return",
    tag = "Rental Bookings",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "Rental booking ID")),
    responses(
        (status = 200, description = "Return validated", body = RentalBookingResponse),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found"),
    )
)]
pub async fn validate_return(
    auth: AuthSeller,
    Path(id): Path<i32>,
    State(state): State<AppState>,
    Json(payload): Json<ValidateReturnRequest>,
) -> Result<Json<RentalBookingResponse>, AppError> {
    let rental = rental_repo::find_rental_by_id(&state.db, id)
        .await?
        .ok_or_else(|| AppError::not_found("Rental booking tidak ditemukan"))?;

    if rental.seller_id != auth.user_id {
        return Err(AppError::forbidden("Anda bukan seller dari vehicle ini"));
    }

    if rental.status != "berjalan" {
        return Err(AppError::bad_request("Status rental tidak valid untuk return"));
    }

    // Log notes if provided
    if let Some(notes) = &payload.notes {
        tracing::info!("Return notes untuk rental {}: {}", id, notes);
    }

    let updated = rental_repo::validate_return(&state.db, id).await?;

    tracing::info!("Rental {} return validated by seller {}", id, auth.user_id);

    Ok(Json(RentalBookingResponse::from(updated)))
}

// Cancel rental booking
#[utoipa::path(
    delete,
    path = "/api/rentals/bookings/{id}",
    tag = "Rental Bookings",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "Rental booking ID")),
    request_body = CancelRentalRequest,
    responses(
        (status = 200, description = "Rental cancelled", body = MessageResponse),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found"),
    )
)]
pub async fn cancel_rental_booking(
    auth: AuthCustomer,
    Path(id): Path<i32>,
    State(state): State<AppState>,
    Json(payload): Json<CancelRentalRequest>,
) -> Result<Json<MessageResponse>, AppError> {
    let rental = rental_repo::find_rental_by_id(&state.db, id)
        .await?
        .ok_or_else(|| AppError::not_found("Rental booking tidak ditemukan"))?;

    if rental.customer_id != auth.user_id {
        return Err(AppError::forbidden("Anda tidak memiliki akses ke booking ini"));
    }

    if rental.status == "selesai" || rental.status == "cancelled" {
        return Err(AppError::bad_request("Rental sudah selesai atau dibatalkan"));
    }

    rental_repo::cancel_rental(&state.db, id, &payload.cancel_reason).await?;

    tracing::info!("Rental {} cancelled by customer {}", id, auth.user_id);

    Ok(Json(MessageResponse {
        message: "Rental booking berhasil dibatalkan".to_string(),
    }))
}

// Update rental status (internal use)
#[utoipa::path(
    put,
    path = "/api/rentals/bookings/{id}/status",
    tag = "rental-bookings",
    summary = "Update status booking rental",
    description = "Update status booking rental (untuk internal use)",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = i64, Path, description = "ID booking rental")
    ),
    request_body = UpdateRentalStatusRequest,
    responses(
        (status = 200, description = "Status berhasil diupdate", body = RentalBookingResponse),
        (status = 400, description = "Status tidak valid"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Booking tidak ditemukan")
    )
)]
pub async fn update_rental_booking_status(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    auth: AuthUser,
    Json(payload): Json<UpdateRentalStatusRequest>,
) -> Result<Json<RentalBookingResponse>, AppError> {
    // Cek booking ada
    let rental = rental_repo::find_rental_by_id(&state.db, id as i32)
        .await?
        .ok_or(AppError::NotFound("Booking tidak ditemukan".to_string()))?;

    // Validasi akses (hanya customer atau seller yang terkait)
    if auth.user_id != rental.customer_id && auth.user_id != rental.seller_id {
        return Err(AppError::Forbidden("Akses ditolak".to_string()));
    }

    // Validasi status baru menggunakan RentalStatus enum
    if RentalStatus::from_str(&payload.status).is_none() {
        return Err(AppError::BadRequest("Status tidak valid".to_string()));
    }

    // Update status
    let updated = rental_repo::update_rental_status(
        &state.db,
        id as i32,
        &payload.status,
    ).await?;

    Ok(Json(RentalBookingResponse::from(updated)))
}

// Validasi create rental request
fn validate_create_rental(payload: &CreateRentalRequest) -> Result<(), AppError> {
    if payload.customer_name.trim().is_empty() {
        return Err(AppError::validation("Nama customer harus diisi"));
    }

    if !validation::is_valid_phone(&payload.customer_phone) {
        return Err(AppError::validation("Format nomor telepon tidak valid"));
    }

    if !validation::is_valid_email(&payload.customer_email) {
        return Err(AppError::validation("Format email tidak valid"));
    }

    if payload.pickup_date >= payload.return_date {
        return Err(AppError::validation("Tanggal return harus setelah tanggal pickup"));
    }

    let now = chrono::Utc::now();
    if payload.pickup_date < now {
        return Err(AppError::validation("Tanggal pickup tidak boleh di masa lalu"));
    }

    Ok(())
}