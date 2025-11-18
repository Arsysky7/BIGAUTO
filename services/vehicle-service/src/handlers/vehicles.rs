use axum::{extract::{Path, Query, State}, Json};
use serde::Serialize;
use sqlx::PgPool;
use utoipa::ToSchema;

use crate::{
    config::AppConfig,
    domain::vehicle::{
        VehicleResponse, VehicleListResponse, VehicleFilter,
        CreateVehicleRequest, UpdateVehicleRequest,
    },
    error::AppError,
    middleware::auth::AuthSeller,
    repositories::vehicle_repo,
};

// Import shared validation utilities
use shared::utils::validation;

#[derive(Debug, Serialize, ToSchema)]
pub struct MessageResponse {
    pub message: String,
}

// List vehicles dengan filtering
#[utoipa::path(
    get,
    path = "/api/vehicles",
    tag = "Vehicles",
    params(VehicleFilter),
    responses(
        (status = 200, description = "List vehicles", body = VehicleListResponse),
    )
)]
pub async fn list_vehicles(
    Query(filter): Query<VehicleFilter>,
    State(pool): State<PgPool>,
) -> Result<Json<VehicleListResponse>, AppError> {
    let page = filter.page.unwrap_or(1);
    let limit = filter.limit.unwrap_or(20);

    let (vehicles, total) = vehicle_repo::find_vehicles(&pool, &filter).await?;

    let data: Vec<VehicleResponse> = vehicles
        .into_iter()
        .map(|v| map_to_response_from_with_seller(v))
        .collect();

    let total_pages = (total as f64 / limit as f64).ceil() as i64;

    Ok(Json(VehicleListResponse {
        data,
        total,
        page,
        limit,
        total_pages,
    }))    
}

// Detail vehicle by ID
#[utoipa::path(
    get,
    path = "/api/vehicles/{id}",
    tag = "Vehicles",
    params(("id" = i32, Path, description = "Vehicle ID")),
    responses(
        (status = 200, description = "Vehicle detail", body = VehicleResponse),
        (status = 404, description = "Vehicle tidak ditemukan"),
    )
)]
pub async fn get_vehicle(
    Path(id): Path<i32>,
    State(pool): State<PgPool>,
) -> Result<Json<VehicleResponse>, AppError> {
    let result = vehicle_repo::find_vehicle_by_id(&pool, id).await?;

    match result {
        Some(vehicle) => Ok(Json(map_to_response_from_with_seller(vehicle))),
        None => Err(AppError::not_found("Vehicle tidak ditemukan")),
    }
}

// Create vehicle baru
#[utoipa::path(
    post,
    path = "/api/vehicles",
    tag = "Vehicles",
    security(("bearer_auth" = [])),
    request_body = CreateVehicleRequest,
    responses(
        (status = 201, description = "Vehicle created", body = VehicleResponse),
        (status = 400, description = "Input tidak valid"),
        (status = 403, description = "Hanya seller"),
    )
)]
pub async fn create_vehicle(
    auth: AuthSeller,
    State(pool): State<PgPool>,
    State(config): State<AppConfig>,
    Json(payload): Json<CreateVehicleRequest>,
) -> Result<Json<VehicleResponse>, AppError> {
    // Audit log: siapa yang create vehicle
    tracing::info!(
        "Seller {} ({}) creating vehicle: {}",
        auth.user_id,
        auth.email,
        payload.title
    );

    validate_create_request(&payload, &config)?;

    let vehicle = vehicle_repo::create_vehicle(&pool, auth.user_id, &payload).await?;
    let seller_name = vehicle_repo::find_seller_name(&pool, auth.user_id).await?;

    tracing::info!(
        "Vehicle {} created by seller {} ({})",
        vehicle.id,
        auth.user_id,
        auth.email
    );

    Ok(Json(map_to_response(vehicle, seller_name)))
}

// Update vehicle
#[utoipa::path(
    put,
    path = "/api/vehicles/{id}",
    tag = "Vehicles",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "Vehicle ID")),
    request_body = UpdateVehicleRequest,
    responses(
        (status = 200, description = "Vehicle updated", body = VehicleResponse),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found"),
    )
)]
pub async fn update_vehicle(
    auth: AuthSeller,
    Path(id): Path<i32>,
    State(pool): State<PgPool>,
    Json(payload): Json<UpdateVehicleRequest>,
) -> Result<Json<VehicleResponse>, AppError> {
    // Audit log: siapa yang update vehicle
    tracing::info!(
        "Seller {} ({}) updating vehicle {}",
        auth.user_id,
        auth.email,
        id
    );

    vehicle_repo::check_ownership(&pool, id, auth.user_id).await?;

    if !has_update_fields(&payload) {
        return Err(AppError::bad_request("Tidak ada field yang diupdate"));
    }

    let vehicle = vehicle_repo::update_vehicle(&pool, id, &payload).await?;
    let seller_name = vehicle_repo::find_seller_name(&pool, auth.user_id).await?;

    tracing::info!(
        "Vehicle {} updated by seller {} ({})",
        id,
        auth.user_id,
        auth.email
    );

    Ok(Json(map_to_response(vehicle, seller_name)))
}

// Delete vehicle
#[utoipa::path(
    delete,
    path = "/api/vehicles/{id}",
    tag = "Vehicles",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "Vehicle ID")),
    responses(
        (status = 200, description = "Vehicle deleted", body = MessageResponse),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found"),
    )
)]
pub async fn delete_vehicle(
    auth: AuthSeller,
    Path(id): Path<i32>,
    State(pool): State<PgPool>,
) -> Result<Json<MessageResponse>, AppError> {
    // Audit log: siapa yang delete vehicle
    tracing::warn!(
        "Seller {} ({}) deleting vehicle {}",
        auth.user_id,
        auth.email,
        id
    );

    vehicle_repo::check_ownership(&pool, id, auth.user_id).await?;
    vehicle_repo::delete_vehicle(&pool, id).await?;

    tracing::info!(
        "Vehicle {} deleted by seller {} ({})",
        id,
        auth.user_id,
        auth.email
    );

    Ok(Json(MessageResponse {
        message: "Vehicle berhasil dihapus".to_string(),
    }))
}

// Map Vehicle ke Response (full data)
pub fn map_to_response(v: crate::domain::vehicle::Vehicle, seller_name: String) -> VehicleResponse {
    let photos: Vec<String> = serde_json::from_value(v.photos.clone()).unwrap_or_default();

    VehicleResponse {
        id: v.id,
        seller_id: v.seller_id,
        seller_name,
        title: v.title,
        category: v.category,
        price: v.price,
        brand: v.brand,
        model: v.model,
        year: v.year,
        transmission: v.transmission,
        fuel_type: v.fuel_type,
        engine_capacity: v.engine_capacity,
        mileage: v.mileage,
        seats: v.seats,
        doors: v.doors,
        luggage_capacity: v.luggage_capacity,
        vehicle_type: v.vehicle_type,
        is_luxury: v.is_luxury,
        is_flood_free: v.is_flood_free,
        tax_active: v.tax_active,
        has_bpkb: v.has_bpkb,
        has_stnk: v.has_stnk,
        description: v.description,
        rental_terms: v.rental_terms,
        city: v.city,
        address: v.address,
        latitude: v.latitude,
        longitude: v.longitude,
        area_coverage: v.area_coverage,
        photos,
        status: v.status,
        rating: v.rating,
        review_count: v.review_count,
        created_at: v.created_at,
        updated_at: v.updated_at,
    }
}

// Map VehicleWithSeller ke Response (full data)
fn map_to_response_from_with_seller(v: crate::domain::vehicle::VehicleWithSeller) -> VehicleResponse {
    let photos: Vec<String> = serde_json::from_value(v.photos.clone()).unwrap_or_default();

    VehicleResponse {
        id: v.id,
        seller_id: v.seller_id,
        seller_name: v.seller_name,
        title: v.title,
        category: v.category,
        price: v.price,
        brand: v.brand,
        model: v.model,
        year: v.year,
        transmission: v.transmission,
        fuel_type: v.fuel_type,
        engine_capacity: v.engine_capacity,
        mileage: v.mileage,
        seats: v.seats,
        doors: v.doors,
        luggage_capacity: v.luggage_capacity,
        vehicle_type: v.vehicle_type,
        is_luxury: v.is_luxury,
        is_flood_free: v.is_flood_free,
        tax_active: v.tax_active,
        has_bpkb: v.has_bpkb,
        has_stnk: v.has_stnk,
        description: v.description,
        rental_terms: v.rental_terms,
        city: v.city,
        address: v.address,
        latitude: v.latitude,
        longitude: v.longitude,
        area_coverage: v.area_coverage,
        photos,
        status: v.status,
        rating: v.rating,
        review_count: v.review_count,
        created_at: v.created_at,
        updated_at: v.updated_at,
    }
}

// Validasi create request dengan strict mode untuk production
fn validate_create_request(req: &CreateVehicleRequest, config: &AppConfig) -> Result<(), AppError> {
    // Validate title (business rule - gunakan validation error)
    let title = validation::sanitize_html(&req.title);
    if title.trim().is_empty() {
        return Err(AppError::validation("Title tidak boleh kosong"));
    }

    // Validate category (business rule)
    if req.category != "rental" && req.category != "sale" {
        return Err(AppError::validation("Category harus rental atau sale"));
    }

    // Validate price using shared validation (business rule)
    if !validation::is_valid_price(req.price as i64) {
        return Err(AppError::validation("Price tidak valid (harus 1 - 10 miliar)"));
    }

    // Validate year using shared validation (business rule)
    if !validation::is_valid_year(req.year) {
        return Err(AppError::validation("Year tidak valid (1900 - tahun depan)"));
    }

    // Validate seats (business rule)
    if req.seats < 1 || req.seats > 50 {
        return Err(AppError::validation("Seats harus antara 1-50"));
    }

    // Validate & sanitize description (business rule)
    if let Some(ref desc) = req.description {
        let sanitized = validation::sanitize_html(desc);
        if sanitized.len() > 5000 {
            return Err(AppError::validation("Description terlalu panjang (max 5000 karakter)"));
        }
    }

    // Strict validation untuk production: lebih ketat requirement photos
    if config.strict_validation() {
        // Production mode: minimal 5 photos untuk semua kategori
        if req.photos.len() < 5 {
            return Err(AppError::validation(
                "Production mode memerlukan minimal 5 foto berkualitas tinggi"
            ));
        }

        // Production mode: wajib ada description
        if req.description.is_none() || req.description.as_ref().map(|d| d.trim().is_empty()).unwrap_or(true) {
            return Err(AppError::validation(
                "Production mode memerlukan description yang lengkap"
            ));
        }

        // Production mode: untuk sale, wajib ada informasi detail
        if req.category == "sale" {
            if req.mileage.is_none() {
                return Err(AppError::validation(
                    "Sale vehicle harus mencantumkan mileage di production mode"
                ));
            }
            if req.transmission.is_none() {
                return Err(AppError::validation(
                    "Sale vehicle harus mencantumkan transmission di production mode"
                ));
            }
        }
    } else {
        // Development mode: lebih lenient
        let min_photos = if req.category == "sale" { 5 } else { 3 };
        if req.photos.len() < min_photos {
            return Err(AppError::validation(format!(
                "Minimal {} foto untuk {}",
                min_photos, req.category
            )));
        }
    }

    Ok(())
}

// Check apakah ada field yang diupdate
fn has_update_fields(req: &UpdateVehicleRequest) -> bool {
    req.title.is_some()
        || req.price.is_some()
        || req.transmission.is_some()
        || req.fuel_type.is_some()
        || req.engine_capacity.is_some()
        || req.mileage.is_some()
        || req.seats.is_some()
        || req.doors.is_some()
        || req.luggage_capacity.is_some()
        || req.description.is_some()
        || req.rental_terms.is_some()
        || req.address.is_some()
        || req.latitude.is_some()
        || req.longitude.is_some()
}
