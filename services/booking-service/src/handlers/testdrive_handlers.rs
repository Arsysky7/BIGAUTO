use axum::{extract::{Path, Query, State}, Json};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    domain::testdrive::{
        TestDriveBookingResponse, CreateTestDriveRequest,
        RescheduleTestDriveRequest, ChooseRescheduleSlotRequest,
        CancelTestDriveRequest, ConfirmTestDriveRequest,
        CompleteTestDriveRequest, TestDriveStatus,
    },
    error::AppError,
    repositories::testdrive_repo,
    AppState,
};

use crate::middleware::auth::{AuthUser, AuthCustomer, AuthSeller};
use shared::utils::validation;

#[derive(Debug, Serialize, ToSchema)]
pub struct MessageResponse {
    pub message: String,
}

#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct TestDriveFilter {
    pub status: Option<String>,
}

// Create test drive booking baru (customer)
#[utoipa::path(
    post,
    path = "/api/testdrives/bookings",
    tag = "Test Drive Bookings",
    security(("bearer_auth" = [])),
    request_body = CreateTestDriveRequest,
    responses(
        (status = 201, description = "Test drive booking created", body = TestDriveBookingResponse),
        (status = 400, description = "Input tidak valid"),
    )
)]
pub async fn create_testdrive_booking(
    auth: AuthCustomer,
    State(state): State<AppState>,
    Json(payload): Json<CreateTestDriveRequest>,
) -> Result<Json<TestDriveBookingResponse>, AppError> {
    tracing::info!(
        "Customer {} ({}) creating testdrive for vehicle {}",
        auth.user_id,
        auth.email,
        payload.vehicle_id
    );

    // Validasi input
    validate_create_testdrive(&payload)?;

    // Check vehicle exists dan ambil seller_id dari vehicle-service (harus jual-beli)
    let url = format!("{}/vehicles/{}/testdrive-info",
        state.config.vehicle_service_url,
        payload.vehicle_id
    );

    #[derive(serde::Deserialize)]
    struct VehicleTestDriveInfo {
        id: i32,
        seller_id: i32,
        is_available: bool,
    }

    let response = state.http_client
        .get(&url)
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| AppError::database_error(format!("Gagal menghubungi vehicle-service: {}", e)))?;

    if !response.status().is_success() {
        return Err(AppError::not_found("Vehicle tidak tersedia untuk test drive"));
    }

    let vehicle_info: VehicleTestDriveInfo = response
        .json()
        .await
        .map_err(|e| AppError::database_error(format!("Gagal parse response: {}", e)))?;

    if !vehicle_info.is_available {
        return Err(AppError::bad_request("Vehicle tidak tersedia untuk test drive"));
    }

    let (_vehicle_id, seller_id) = (vehicle_info.id, vehicle_info.seller_id);

    // Create test drive booking
    let testdrive = testdrive_repo::create_testdrive(
        &state.db,
        auth.user_id,
        seller_id,
        &payload,
    ).await?;

    tracing::info!("Test drive booking {} created", testdrive.id);

    Ok(Json(TestDriveBookingResponse::from(testdrive)))
}

// Get test drive booking by ID
#[utoipa::path(
    get,
    path = "/api/testdrives/bookings/{id}",
    tag = "Test Drive Bookings",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "Test drive booking ID")),
    responses(
        (status = 200, description = "Test drive booking detail", body = TestDriveBookingResponse),
        (status = 404, description = "Test drive booking tidak ditemukan"),
    )
)]
pub async fn get_testdrive_booking(
    auth: AuthUser,
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<TestDriveBookingResponse>, AppError> {
    let testdrive = testdrive_repo::find_testdrive_by_id(&state.db, id)
        .await?
        .ok_or_else(|| AppError::not_found("Test drive booking tidak ditemukan"))?;

    // Check authorization (customer or seller)
    if testdrive.customer_id != auth.user_id && testdrive.seller_id != auth.user_id {
        return Err(AppError::forbidden("Anda tidak memiliki akses ke booking ini"));
    }

    Ok(Json(TestDriveBookingResponse::from(testdrive)))
}

// List my test drive bookings (customer)
#[utoipa::path(
    get,
    path = "/api/testdrives/bookings/my",
    tag = "Test Drive Bookings",
    security(("bearer_auth" = [])),
    params(TestDriveFilter),
    responses(
        (status = 200, description = "List test drive bookings", body = Vec<TestDriveBookingResponse>),
    )
)]
pub async fn get_customer_testdrive_bookings(
    auth: AuthCustomer,
    Query(filter): Query<TestDriveFilter>,
    State(state): State<AppState>,
) -> Result<Json<Vec<TestDriveBookingResponse>>, AppError> {
    let testdrives = testdrive_repo::find_testdrives_by_customer(
        &state.db,
        auth.user_id,
        filter.status,
    ).await?;

    let response: Vec<TestDriveBookingResponse> = testdrives
        .into_iter()
        .map(TestDriveBookingResponse::from)
        .collect();

    Ok(Json(response))
}

// List seller test drive bookings (seller)
#[utoipa::path(
    get,
    path = "/api/testdrives/bookings/seller",
    tag = "Test Drive Bookings",
    security(("bearer_auth" = [])),
    params(TestDriveFilter),
    responses(
        (status = 200, description = "List test drive bookings", body = Vec<TestDriveBookingResponse>),
    )
)]
pub async fn get_seller_testdrive_bookings(
    auth: AuthSeller,
    Query(filter): Query<TestDriveFilter>,
    State(state): State<AppState>,
) -> Result<Json<Vec<TestDriveBookingResponse>>, AppError> {
    let testdrives = testdrive_repo::find_testdrives_by_seller(
        &state.db,
        auth.user_id,
        filter.status,
    ).await?;

    let response: Vec<TestDriveBookingResponse> = testdrives
        .into_iter()
        .map(TestDriveBookingResponse::from)
        .collect();

    Ok(Json(response))
}

// Seller accept test drive booking
#[utoipa::path(
    put,
    path = "/api/testdrives/bookings/{id}/accept",
    tag = "Test Drive Bookings",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "Test drive booking ID")),
    responses(
        (status = 200, description = "Test drive accepted", body = TestDriveBookingResponse),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found"),
    )
)]
pub async fn accept_testdrive_booking(
    auth: AuthSeller,
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<TestDriveBookingResponse>, AppError> {
    let testdrive = testdrive_repo::find_testdrive_by_id(&state.db, id)
        .await?
        .ok_or_else(|| AppError::not_found("Test drive booking tidak ditemukan"))?;

    if testdrive.seller_id != auth.user_id {
        return Err(AppError::forbidden("Anda bukan seller dari vehicle ini"));
    }

    if testdrive.status != "menunggu_konfirmasi" {
        return Err(AppError::bad_request("Test drive tidak dalam status menunggu konfirmasi"));
    }

    // Accept test drive booking
    let updated = testdrive_repo::confirm_testdrive(&state.db, id).await?;

    tracing::info!("Test drive {} accepted by seller {}", id, auth.user_id);

    Ok(Json(TestDriveBookingResponse::from(updated)))
}

// Seller reschedule test drive dengan alternative slots
#[utoipa::path(
    put,
    path = "/api/testdrives/bookings/{id}/reschedule",
    tag = "Test Drive Bookings",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "Test drive booking ID")),
    request_body = RescheduleTestDriveRequest,
    responses(
        (status = 200, description = "Test drive rescheduled", body = TestDriveBookingResponse),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found"),
    )
)]
pub async fn reschedule_testdrive_booking(
    auth: AuthSeller,
    Path(id): Path<i32>,
    State(state): State<AppState>,
    Json(payload): Json<RescheduleTestDriveRequest>,
) -> Result<Json<TestDriveBookingResponse>, AppError> {
    let testdrive = testdrive_repo::find_testdrive_by_id(&state.db, id)
        .await?
        .ok_or_else(|| AppError::not_found("Test drive booking tidak ditemukan"))?;

    if testdrive.seller_id != auth.user_id {
        return Err(AppError::forbidden("Anda bukan seller dari vehicle ini"));
    }

    if testdrive.status != "menunggu_konfirmasi" {
        return Err(AppError::bad_request("Test drive tidak dalam status menunggu konfirmasi"));
    }

    let reschedule_slots: sqlx::types::JsonValue = serde_json::to_value(&payload.reschedule_slots)
        .map_err(|_| AppError::internal("Invalid reschedule slots format"))?;

    let updated = testdrive_repo::reschedule_testdrive(&state.db, id, reschedule_slots).await?;

    tracing::info!("Test drive {} rescheduled by seller {}", id, auth.user_id);

    Ok(Json(TestDriveBookingResponse::from(updated)))
}

// Customer pilih slot reschedule
#[utoipa::path(
    put,
    path = "/api/testdrives/bookings/{id}/choose-slot",
    tag = "Test Drive Bookings",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "Test drive booking ID")),
    request_body = ChooseRescheduleSlotRequest,
    responses(
        (status = 200, description = "Slot chosen", body = TestDriveBookingResponse),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found"),
    )
)]
pub async fn choose_reschedule_slot(
    auth: AuthCustomer,
    Path(id): Path<i32>,
    State(state): State<AppState>,
    Json(payload): Json<ChooseRescheduleSlotRequest>,
) -> Result<Json<TestDriveBookingResponse>, AppError> {
    let testdrive = testdrive_repo::find_testdrive_by_id(&state.db, id)
        .await?
        .ok_or_else(|| AppError::not_found("Test drive booking tidak ditemukan"))?;

    if testdrive.customer_id != auth.user_id {
        return Err(AppError::forbidden("Anda tidak memiliki akses ke booking ini"));
    }

    if testdrive.status != "seller_reschedule" {
        return Err(AppError::bad_request("Test drive tidak dalam status reschedule"));
    }

    let updated = testdrive_repo::choose_reschedule_slot(&state.db, id, payload.slot_index).await?;

    tracing::info!("Customer {} chose slot {} for testdrive {}", auth.user_id, payload.slot_index, id);

    Ok(Json(TestDriveBookingResponse::from(updated)))
}

// Seller confirm test drive
#[utoipa::path(
    put,
    path = "/api/testdrives/bookings/{id}/confirm",
    tag = "Test Drive Bookings",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "Test drive booking ID")),
    request_body = ConfirmTestDriveRequest,
    responses(
        (status = 200, description = "Test drive confirmed", body = TestDriveBookingResponse),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found"),
    )
)]
pub async fn confirm_testdrive_booking(
    auth: AuthSeller,
    Path(id): Path<i32>,
    State(state): State<AppState>,
    Json(payload): Json<ConfirmTestDriveRequest>,
) -> Result<Json<TestDriveBookingResponse>, AppError> {
    let testdrive = testdrive_repo::find_testdrive_by_id(&state.db, id)
        .await?
        .ok_or_else(|| AppError::not_found("Test drive booking tidak ditemukan"))?;

    if testdrive.seller_id != auth.user_id {
        return Err(AppError::forbidden("Anda bukan seller dari vehicle ini"));
    }

    if TestDriveStatus::from_str(&testdrive.status) != Some(TestDriveStatus::MenungguKonfirmasi) {
        return Err(AppError::bad_request("Test drive tidak dalam status menunggu konfirmasi"));
    }

    // Validasi status dari payload menggunakan TestDriveStatus enum
    if TestDriveStatus::from_str(&payload.status) != Some(TestDriveStatus::Diterima) {
        return Err(AppError::bad_request("Status tidak valid. Hanya 'diterima' yang diperbolehkan"));
    }

    let updated = testdrive_repo::confirm_testdrive(&state.db, id).await?;

    tracing::info!("Test drive {} confirmed by seller {}", id, auth.user_id);

    Ok(Json(TestDriveBookingResponse::from(updated)))
}

// Seller complete test drive
#[utoipa::path(
    put,
    path = "/api/testdrives/bookings/{id}/complete",
    tag = "Test Drive Bookings",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "Test drive booking ID")),
    request_body = CompleteTestDriveRequest,
    responses(
        (status = 200, description = "Test drive completed", body = TestDriveBookingResponse),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found"),
    )
)]
pub async fn complete_testdrive_booking(
    auth: AuthSeller,
    Path(id): Path<i32>,
    State(state): State<AppState>,
    Json(payload): Json<CompleteTestDriveRequest>,
) -> Result<Json<TestDriveBookingResponse>, AppError> {
    let testdrive = testdrive_repo::find_testdrive_by_id(&state.db, id)
        .await?
        .ok_or_else(|| AppError::not_found("Test drive booking tidak ditemukan"))?;

    if testdrive.seller_id != auth.user_id {
        return Err(AppError::forbidden("Anda bukan seller dari vehicle ini"));
    }

    if testdrive.status != "diterima" {
        return Err(AppError::bad_request("Test drive belum diterima"));
    }

    // Log notes jika provided
    if let Some(notes) = &payload.notes {
        tracing::info!("Completion notes untuk test drive {}: {}", id, notes);
    }

    let updated = testdrive_repo::complete_testdrive(&state.db, id).await?;

    tracing::info!("Test drive {} completed by seller {}", id, auth.user_id);

    Ok(Json(TestDriveBookingResponse::from(updated)))
}

// Cancel test drive booking
#[utoipa::path(
    delete,
    path = "/api/testdrives/bookings/{id}",
    tag = "Test Drive Bookings",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "Test drive booking ID")),
    request_body = CancelTestDriveRequest,
    responses(
        (status = 200, description = "Test drive cancelled", body = MessageResponse),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found"),
    )
)]
pub async fn cancel_testdrive_booking(
    auth: AuthCustomer,
    Path(id): Path<i32>,
    State(state): State<AppState>,
    Json(payload): Json<CancelTestDriveRequest>,
) -> Result<Json<MessageResponse>, AppError> {
    let testdrive = testdrive_repo::find_testdrive_by_id(&state.db, id)
        .await?
        .ok_or_else(|| AppError::not_found("Test drive booking tidak ditemukan"))?;

    if testdrive.customer_id != auth.user_id {
        return Err(AppError::forbidden("Anda tidak memiliki akses ke booking ini"));
    }

    if testdrive.status == "selesai" || testdrive.status == "cancelled" {
        return Err(AppError::bad_request("Test drive sudah selesai atau dibatalkan"));
    }

    testdrive_repo::cancel_testdrive(&state.db, id, &payload.cancel_reason).await?;

    tracing::info!("Test drive {} cancelled by customer {}", id, auth.user_id);

    Ok(Json(MessageResponse {
        message: "Test drive booking berhasil dibatalkan".to_string(),
    }))
}

// Auto-timeout expired test drives (scheduler endpoint)
#[utoipa::path(
    post,
    path = "/api/testdrives/timeout-expired",
    tag = "testdrive-bookings",
    summary = "Timeout test drives yang kadaluarsa",
    description = "Scheduler function untuk menandai test drives yang kadaluarsa",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Jumlah test drives yang ditimeout"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn timeout_expired_testdrives(
    State(state): State<AppState>,
    auth: AuthUser, 
) -> Result<Json<serde_json::Value>, AppError> {
    let timeout_count = testdrive_repo::timeout_expired_testdrives(&state.db).await?;

    tracing::info!("Timed out {} expired test drives by user {}", timeout_count, auth.user_id);

    Ok(Json(serde_json::json!({
        "timeout_count": timeout_count,
        "message": format!("{} test drives berhasil ditimeout", timeout_count)
    })))
}

// Validasi create testdrive request
fn validate_create_testdrive(payload: &CreateTestDriveRequest) -> Result<(), AppError> {
    if payload.customer_name.trim().is_empty() {
        return Err(AppError::validation("Nama customer harus diisi"));
    }

    if !validation::is_valid_phone(&payload.customer_phone) {
        return Err(AppError::validation("Format nomor telepon tidak valid"));
    }

    if !validation::is_valid_email(&payload.customer_email) {
        return Err(AppError::validation("Format email tidak valid"));
    }

    let now = chrono::Utc::now();
    if payload.requested_date < now {
        return Err(AppError::validation("Tanggal test drive tidak boleh di masa lalu"));
    }

    Ok(())
}
