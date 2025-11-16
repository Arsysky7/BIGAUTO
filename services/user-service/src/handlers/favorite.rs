use axum::{
    extract::{Path, State},
    Json,
};
use serde::Serialize;
use serde_json::Value as JsonValue;
use shared::utils::http_client::ServiceClient;
use sqlx::PgPool;
use utoipa::ToSchema;

use crate::{
    config::AppConfig,
    domain::favorite::{Favorite, AddFavoriteRequest, FavoriteWithVehicle, CheckFavoriteResponse},
    error::AppError,
    middleware::AuthUser,
};

// Response sukses dengan message
#[derive(Debug, Serialize, ToSchema)]
pub struct MessageResponse {
    pub message: String,
}

// Ambil semua favorites user
#[utoipa::path(
    get,
    path = "/api/users/me/favorites",
    tag = "Favorites",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List favorites berhasil diambil", body = Vec<FavoriteWithVehicle>),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn get_favorites(
    auth: AuthUser,
    State(pool): State<PgPool>,
) -> Result<Json<Vec<FavoriteWithVehicle>>, AppError> {
    let favorites = fetch_user_favorites(&pool, auth.user_id).await?;

    // Fetch vehicle info untuk setiap favorite
    let mut result = Vec::new();
    for fav in favorites {
        if let Ok(vehicle) = get_vehicle_info(fav.vehicle_id).await {
            result.push(FavoriteWithVehicle {
                id: fav.id,
                vehicle_id: fav.vehicle_id,
                created_at: fav.created_at,
                vehicle_title: extract_string(&vehicle, "title"),
                vehicle_price: extract_i64(&vehicle, "price_per_day")
                    .or_else(|| extract_i64(&vehicle, "price"))
                    .unwrap_or(0),
                vehicle_photo: extract_optional_string(&vehicle, "photos")
                    .and_then(|s| parse_first_photo(&s)),
                vehicle_city: extract_string(&vehicle, "city"),
            });
        }
    }

    Ok(Json(result))
}

// Tambah favorite
#[utoipa::path(
    post,
    path = "/api/users/me/favorites",
    tag = "Favorites",
    security(("bearer_auth" = [])),
    request_body = AddFavoriteRequest,
    responses(
        (status = 200, description = "Berhasil menambahkan favorite", body = Favorite),
        (status = 400, description = "Sudah ada di favorites"),
        (status = 401, description = "Unauthorized"),
        (status = 422, description = "Limit favorites tercapai (production)"),
    )
)]
pub async fn add_favorite(
    auth: AuthUser,
    State(pool): State<PgPool>,
    State(config): State<AppConfig>,
    Json(payload): Json<AddFavoriteRequest>,
) -> Result<Json<Favorite>, AppError> {
    // Log untuk audit trail
    tracing::info!(
        "User {} ({}) adding vehicle {} to favorites",
        auth.email,
        auth.role,
        payload.vehicle_id
    );

    // Strict validation: limit favorites di production untuk prevent abuse
    if config.strict_validation() {
        let current_count = count_user_favorites(&pool, auth.user_id).await?;
        const MAX_FAVORITES_PRODUCTION: i64 = 50;

        if current_count >= MAX_FAVORITES_PRODUCTION {
            return Err(AppError::validation(format!(
                "Maksimal {} favorites di production mode untuk optimasi performa",
                MAX_FAVORITES_PRODUCTION
            )));
        }
    }

    // Cek apakah vehicle exist
    get_vehicle_info(payload.vehicle_id).await
        .map_err(|_| AppError::not_found("Vehicle tidak ditemukan"))?;

    // Insert ke database
    let favorite = insert_favorite(&pool, auth.user_id, payload.vehicle_id).await?;
    Ok(Json(favorite))
}

// Hapus favorite
#[utoipa::path(
    delete,
    path = "/api/users/me/favorites/{vehicle_id}",
    tag = "Favorites",
    security(("bearer_auth" = [])),
    params(
        ("vehicle_id" = i32, Path, description = "Vehicle ID")
    ),
    responses(
        (status = 200, description = "Berhasil menghapus favorite", body = MessageResponse),
        (status = 404, description = "Favorite tidak ditemukan"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn remove_favorite(
    auth: AuthUser,
    Path(vehicle_id): Path<i32>,
    State(pool): State<PgPool>,
) -> Result<Json<MessageResponse>, AppError> {
    delete_favorite(&pool, auth.user_id, vehicle_id).await?;
    Ok(Json(MessageResponse {
        message: "Favorite berhasil dihapus".to_string(),
    }))
}


// Cek apakah vehicle sudah di-favorite
#[utoipa::path(
    get,
    path = "/api/users/me/favorites/check/{vehicle_id}",
    tag = "Favorites",
    security(("bearer_auth" = [])),
    params(
        ("vehicle_id" = i32, Path, description = "Vehicle ID")
    ),
    responses(
        (status = 200, description = "Status favorite", body = CheckFavoriteResponse),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn check_favorite(
    auth: AuthUser,
    Path(vehicle_id): Path<i32>,
    State(pool): State<PgPool>,
) -> Result<Json<CheckFavoriteResponse>, AppError> {
    let is_favorite = is_vehicle_favorited(&pool, auth.user_id, vehicle_id).await?;
    Ok(Json(CheckFavoriteResponse { is_favorite }))
}

// === Helper Functions ===

// Count total favorites untuk user (untuk strict validation)
async fn count_user_favorites(pool: &PgPool, customer_id: i32) -> Result<i64, AppError> {
    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM favorites WHERE customer_id = $1"
    )
    .bind(customer_id)
    .fetch_one(pool)
    .await?;

    Ok(count.0)
}

async fn fetch_user_favorites(pool: &PgPool, customer_id: i32) -> Result<Vec<Favorite>, AppError> {
    let favorites = sqlx::query_as::<_, Favorite>(
        "SELECT * FROM favorites WHERE customer_id = $1 ORDER BY created_at DESC"
    )
    .bind(customer_id)
    .fetch_all(pool)
    .await?;

    Ok(favorites)
}

// Insert favorite baru
async fn insert_favorite(pool: &PgPool, customer_id: i32, vehicle_id: i32) -> Result<Favorite, AppError> {
    let favorite = sqlx::query_as::<_, Favorite>(
        r#"
        INSERT INTO favorites (customer_id, vehicle_id)
        VALUES ($1, $2)
        RETURNING *
        "#
    )
    .bind(customer_id)
    .bind(vehicle_id)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        if e.to_string().contains("duplicate key") {
            AppError::bad_request("Vehicle sudah ada di favorites")
        } else {
            AppError::Database(e)
        }
    })?;

    Ok(favorite)
}

// Hapus favorite
async fn delete_favorite(pool: &PgPool, customer_id: i32, vehicle_id: i32) -> Result<(), AppError> {
    let result = sqlx::query(
        "DELETE FROM favorites WHERE customer_id = $1 AND vehicle_id = $2"
    )
    .bind(customer_id)
    .bind(vehicle_id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("Favorite tidak ditemukan"));
    }

    Ok(())
}

// Cek apakah vehicle sudah di-favorite
async fn is_vehicle_favorited(pool: &PgPool, customer_id: i32, vehicle_id: i32) -> Result<bool, AppError> {
    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM favorites WHERE customer_id = $1 AND vehicle_id = $2"
    )
    .bind(customer_id)
    .bind(vehicle_id)
    .fetch_one(pool)
    .await?;

    Ok(count.0 > 0)
}

// Fetch vehicle info dari vehicle-service
async fn get_vehicle_info(vehicle_id: i32) -> Result<JsonValue, AppError> {
    let client = ServiceClient::new("vehicle")
        .map_err(|e| AppError::internal(format!("Service client error: {}", e)))?;

    client.get(&format!("/api/vehicles/{}", vehicle_id), None).await
        .map_err(|e| AppError::internal(format!("Failed to fetch vehicle: {}", e)))
}

// Extract string dari JSON
fn extract_string(json: &JsonValue, key: &str) -> String {
    json.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

// Extract optional string dari JSON
fn extract_optional_string(json: &JsonValue, key: &str) -> Option<String> {
    json.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

// Extract i64 dari JSON
fn extract_i64(json: &JsonValue, key: &str) -> Option<i64> {
    json.get(key).and_then(|v| v.as_i64())
}

// Parse first photo dari JSON array
fn parse_first_photo(photos_str: &str) -> Option<String> {
    serde_json::from_str::<Vec<String>>(photos_str)
        .ok()
        .and_then(|arr| arr.first().cloned())
}
