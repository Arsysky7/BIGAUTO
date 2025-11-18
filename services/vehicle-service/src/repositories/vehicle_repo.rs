use sqlx::{PgPool, Row, FromRow};
use serde_json::json;

use crate::{
    domain::vehicle::{Vehicle, VehicleWithSeller, VehicleFilter, CreateVehicleRequest, UpdateVehicleRequest},
    error::AppError,
};

// Ambil list vehicles dengan filtering dan pagination
pub async fn find_vehicles(
    pool: &PgPool,
    filter: &VehicleFilter,
) -> Result<(Vec<VehicleWithSeller>, i64), AppError> {
    let page = filter.page.unwrap_or(1).max(1);
    let limit = filter.limit.unwrap_or(20).min(100);
    let offset = (page - 1) * limit;

    // Static count query dengan semua possible filters
    let count_query = r#"
        SELECT COUNT(*) as count
        FROM vehicles v
        WHERE v.is_available = true
          AND (v.category IS NULL OR v.category = $1)
          AND (v.city IS NULL OR v.city = $2)
          AND (v.brand IS NULL OR v.brand = $3)
          AND (v.model IS NULL OR v.model = $4)
          AND (v.transmission IS NULL OR v.transmission = $5)
          AND (v.fuel_type IS NULL OR v.fuel_type = $6)
          AND (v.vehicle_type IS NULL OR v.vehicle_type = $7)
          AND (v.price >= $8 OR $8 IS NULL)
          AND (v.price <= $9 OR $9 IS NULL)
          AND (v.year >= $10 OR $10 IS NULL)
          AND (v.year <= $11 OR $11 IS NULL)
          AND (v.seats >= $12 OR $12 IS NULL)
          AND (v.is_luxury = $13 OR $13 IS NULL)
    "#;

    let total_result = sqlx::query(count_query)
        .bind(&filter.category)
        .bind(&filter.city)
        .bind(&filter.brand)
        .bind(&filter.model)
        .bind(&filter.transmission)
        .bind(&filter.fuel_type)
        .bind(&filter.vehicle_type)
        .bind(filter.min_price)
        .bind(filter.max_price)
        .bind(filter.min_year)
        .bind(filter.max_year)
        .bind(filter.min_seats)
        .bind(filter.is_luxury)
        .fetch_one(pool)
        .await?;

    let total: i64 = total_result.get::<i64, _>("count");

    // Static query dengan semua possible filters
    let base_query = r#"
        SELECT
            v.id, v.seller_id, v.title, v.category, v.price, v.description, v.photos,
            v.rental_terms, v.city, v.address, v.latitude, v.longitude, v.area_coverage,
            v.status, v.rating, v.review_count, v.created_at, v.updated_at,
            v.brand, v.model, v.year, v.transmission, v.fuel_type, v.engine_capacity,
            v.mileage, v.seats, v.doors, v.luggage_capacity, v.vehicle_type,
            v.is_luxury, v.is_flood_free, v.tax_active, v.has_bpkb, v.has_stnk,
            u.name as seller_name
        FROM vehicles v
        INNER JOIN users u ON v.seller_id = u.id
        WHERE v.status = 'available'
          AND (v.category IS NULL OR v.category = $1)
          AND (v.city IS NULL OR v.city = $2)
          AND (v.brand IS NULL OR v.brand = $3)
          AND (v.model IS NULL OR v.model = $4)
          AND (v.transmission IS NULL OR v.transmission = $5)
          AND (v.fuel_type IS NULL OR v.fuel_type = $6)
          AND (v.vehicle_type IS NULL OR v.vehicle_type = $7)
          AND (v.price >= $8 OR $8 IS NULL)
          AND (v.price <= $9 OR $9 IS NULL)
          AND (v.year >= $10 OR $10 IS NULL)
          AND (v.year <= $11 OR $11 IS NULL)
          AND (v.seats >= $12 OR $12 IS NULL)
          AND (v.is_luxury = $13 OR $13 IS NULL)
    "#;

    let sort = match filter.sort_by.as_deref() {
        Some("price_asc") => "ORDER BY v.price ASC, v.created_at DESC",
        Some("price_desc") => "ORDER BY v.price DESC, v.created_at DESC",
        Some("year_desc") => "ORDER BY v.year DESC, v.created_at DESC",
        _ => "ORDER BY v.created_at DESC",
    };

    let final_query = format!("{} {} LIMIT {} OFFSET {}", base_query, sort, limit, offset);

    #[derive(sqlx::FromRow)]
    struct VehicleWithSellerRow {
        id: i32,
        seller_id: i32,
        title: String,
        category: String,
        price: f64,
        description: Option<String>,
        photos: serde_json::Value,
        rental_terms: Option<String>,
        city: String,
        address: String,
        latitude: Option<f64>,
        longitude: Option<f64>,
        area_coverage: Option<serde_json::Value>,
        status: String,
        rating: Option<f64>,
        review_count: i32,
        created_at: chrono::DateTime<chrono::Utc>,
        updated_at: chrono::DateTime<chrono::Utc>,
        brand: String,
        model: String,
        year: i32,
        transmission: Option<String>,
        fuel_type: Option<String>,
        engine_capacity: Option<i32>,
        mileage: Option<i32>,
        seats: i32,
        doors: Option<i32>,
        luggage_capacity: Option<i32>,
        vehicle_type: String,
        is_luxury: bool,
        is_flood_free: bool,
        tax_active: bool,
        has_bpkb: bool,
        has_stnk: bool,
        seller_name: String,
    }

    let rows = sqlx::query(&final_query)
        .bind(&filter.category)
        .bind(&filter.city)
        .bind(&filter.brand)
        .bind(&filter.model)
        .bind(&filter.transmission)
        .bind(&filter.fuel_type)
        .bind(&filter.vehicle_type)
        .bind(filter.min_price)
        .bind(filter.max_price)
        .bind(filter.min_year)
        .bind(filter.max_year)
        .bind(filter.min_seats)
        .bind(filter.is_luxury)
        .fetch_all(pool)
        .await?;

    let mut vehicles = Vec::new();
    for row in rows {
        let vehicle_row = VehicleWithSellerRow::from_row(&row)?;
        vehicles.push(VehicleWithSeller {
            id: vehicle_row.id,
            seller_id: vehicle_row.seller_id,
            title: vehicle_row.title,
            category: vehicle_row.category,
            price: vehicle_row.price,
            brand: vehicle_row.brand,
            model: vehicle_row.model,
            year: vehicle_row.year,
            transmission: vehicle_row.transmission,
            fuel_type: vehicle_row.fuel_type,
            engine_capacity: vehicle_row.engine_capacity,
            mileage: vehicle_row.mileage,
            seats: vehicle_row.seats,
            doors: vehicle_row.doors,
            luggage_capacity: vehicle_row.luggage_capacity,
            vehicle_type: vehicle_row.vehicle_type,
            is_luxury: vehicle_row.is_luxury,
            is_flood_free: vehicle_row.is_flood_free,
            tax_active: vehicle_row.tax_active,
            has_bpkb: vehicle_row.has_bpkb,
            has_stnk: vehicle_row.has_stnk,
            description: vehicle_row.description,
            rental_terms: vehicle_row.rental_terms,
            city: vehicle_row.city,
            address: vehicle_row.address,
            latitude: vehicle_row.latitude,
            longitude: vehicle_row.longitude,
            area_coverage: vehicle_row.area_coverage,
            photos: vehicle_row.photos,
            status: vehicle_row.status,
            rating: vehicle_row.rating,
            review_count: vehicle_row.review_count,
            created_at: vehicle_row.created_at,
            updated_at: vehicle_row.updated_at,
            seller_name: vehicle_row.seller_name,
        });
    }

    Ok((vehicles, total))
}

// Ambil vehicle by ID dengan seller info
pub async fn find_vehicle_by_id(
    pool: &PgPool,
    id: i32,
) -> Result<Option<VehicleWithSeller>, AppError> {
    let result = sqlx::query_as(
        "SELECT v.*, u.name as seller_name
         FROM vehicles v
         JOIN users u ON v.seller_id = u.id
         WHERE v.id = $1"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(result)
}

// Create vehicle baru
pub async fn create_vehicle(
    pool: &PgPool,
    seller_id: i32,
    payload: &CreateVehicleRequest,
) -> Result<Vehicle, AppError> {
    let photos_json = json!(payload.photos);

    let vehicle = sqlx::query_as(
        "INSERT INTO vehicles (
            seller_id, title, category, price, brand, model, year,
            transmission, fuel_type, engine_capacity, mileage,
            seats, doors, luggage_capacity, vehicle_type, is_luxury,
            is_flood_free, tax_active, has_bpkb, has_stnk,
            description, rental_terms, city, address,
            latitude, longitude, photos
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11,
            $12, $13, $14, $15, $16, $17, $18, $19, $20,
            $21, $22, $23, $24, $25, $26, $27
        ) RETURNING *"
    )
    .bind(seller_id)
    .bind(&payload.title)
    .bind(&payload.category)
    .bind(payload.price)
    .bind(&payload.brand)
    .bind(&payload.model)
    .bind(payload.year)
    .bind(&payload.transmission)
    .bind(&payload.fuel_type)
    .bind(payload.engine_capacity)
    .bind(payload.mileage)
    .bind(payload.seats)
    .bind(payload.doors)
    .bind(payload.luggage_capacity)
    .bind(&payload.vehicle_type)
    .bind(payload.is_luxury.unwrap_or(false))
    .bind(payload.is_flood_free.unwrap_or(false))
    .bind(payload.tax_active.unwrap_or(false))
    .bind(payload.has_bpkb.unwrap_or(false))
    .bind(payload.has_stnk.unwrap_or(false))
    .bind(&payload.description)
    .bind(&payload.rental_terms)
    .bind(&payload.city)
    .bind(&payload.address)
    .bind(payload.latitude)
    .bind(payload.longitude)
    .bind(photos_json)
    .fetch_one(pool)
    .await?;

    Ok(vehicle)
}

// Update vehicle dengan dynamic fields
pub async fn update_vehicle(
    pool: &PgPool,
    id: i32,
    payload: &UpdateVehicleRequest,
) -> Result<Vehicle, AppError> {
    let vehicle = sqlx::query_as(
        "UPDATE vehicles SET
            title = COALESCE($1, title),
            price = COALESCE($2, price),
            transmission = COALESCE($3, transmission),
            fuel_type = COALESCE($4, fuel_type),
            engine_capacity = COALESCE($5, engine_capacity),
            mileage = COALESCE($6, mileage),
            seats = COALESCE($7, seats),
            doors = COALESCE($8, doors),
            luggage_capacity = COALESCE($9, luggage_capacity),
            description = COALESCE($10, description),
            rental_terms = COALESCE($11, rental_terms),
            address = COALESCE($12, address),
            latitude = COALESCE($13, latitude),
            longitude = COALESCE($14, longitude),
            updated_at = NOW()
         WHERE id = $15
         RETURNING *"
    )
    .bind(&payload.title)
    .bind(payload.price)
    .bind(&payload.transmission)
    .bind(&payload.fuel_type)
    .bind(payload.engine_capacity)
    .bind(payload.mileage)
    .bind(payload.seats)
    .bind(payload.doors)
    .bind(payload.luggage_capacity)
    .bind(&payload.description)
    .bind(&payload.rental_terms)
    .bind(&payload.address)
    .bind(payload.latitude)
    .bind(payload.longitude)
    .bind(id)
    .fetch_one(pool)
    .await?;

    Ok(vehicle)
}

// Soft delete vehicle (update status ke sold)
pub async fn delete_vehicle(pool: &PgPool, id: i32) -> Result<(), AppError> {
    sqlx::query("UPDATE vehicles SET status = 'sold', updated_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}

// Update photos vehicle
pub async fn update_photos(
    pool: &PgPool,
    id: i32,
    photos: Vec<String>,
) -> Result<Vehicle, AppError> {
    let photos_json = json!(photos);

    let vehicle = sqlx::query_as(
        "UPDATE vehicles SET photos = $1, updated_at = NOW() WHERE id = $2 RETURNING *"
    )
    .bind(photos_json)
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to update photos for vehicle {}: {:?}", id, e);
        AppError::internal(format!("Gagal update photos: {}", e))
    })?;

    Ok(vehicle)
}

// Ambil seller name by ID
pub async fn find_seller_name(pool: &PgPool, seller_id: i32) -> Result<String, AppError> {
    let result: (String,) = sqlx::query_as("SELECT name FROM users WHERE id = $1")
        .bind(seller_id)
        .fetch_one(pool)
        .await?;

    Ok(result.0)
}

// Check ownership vehicle oleh seller
pub async fn check_ownership(
    pool: &PgPool,
    vehicle_id: i32,
    seller_id: i32,
) -> Result<Vehicle, AppError> {
    let vehicle: Option<Vehicle> = sqlx::query_as("SELECT * FROM vehicles WHERE id = $1")
        .bind(vehicle_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error checking ownership: {:?}", e);
            AppError::internal(format!("Gagal check ownership: {}", e))
        })?;

    let vehicle = vehicle.ok_or_else(|| AppError::not_found("Vehicle tidak ditemukan"))?;

    if vehicle.seller_id != seller_id {
        tracing::warn!(
            "Ownership violation: seller {} tried to access vehicle {} owned by seller {}",
            seller_id,
            vehicle_id,
            vehicle.seller_id
        );
        return Err(AppError::forbidden("Anda tidak punya akses ke vehicle ini"));
    }

    Ok(vehicle)
}

