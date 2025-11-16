use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

// Model favorite dari database
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct Favorite {
    pub id: i32,
    pub customer_id: i32,
    pub vehicle_id: i32,
    pub created_at: DateTime<Utc>,
}

// Request add favorite
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[schema(example = json!({
    "vehicle_id": 123
}))]
pub struct AddFavoriteRequest {
    /// Vehicle ID yang mau di-favorite
    #[schema(example = 123)]
    pub vehicle_id: i32,
}

// Response favorite dengan info vehicle
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "id": 1,
    "vehicle_id": 123,
    "created_at": "2025-01-01T00:00:00Z",
    "vehicle_title": "Toyota Avanza 2023 - Automatic",
    "vehicle_price": 350000,
    "vehicle_photo": "https://res.cloudinary.com/drjf5hd0p/image/upload/v1234/vehicles/avanza.jpg",
    "vehicle_city": "Jakarta"
}))]
pub struct FavoriteWithVehicle {
    pub id: i32,
    pub vehicle_id: i32,
    pub created_at: DateTime<Utc>,
    pub vehicle_title: String,
    pub vehicle_price: i64,
    pub vehicle_photo: Option<String>,
    pub vehicle_city: String,
}

// Response check favorite
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "is_favorite": true
}))]
pub struct CheckFavoriteResponse {
    pub is_favorite: bool,
}