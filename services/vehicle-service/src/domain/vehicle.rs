use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::JsonValue;
use utoipa::ToSchema;

// Model utama Vehicle dari database
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Vehicle {
    pub id: i32,
    pub seller_id: i32,
    pub title: String,
    pub category: String,
    pub price: f64,
    pub brand: String,
    pub model: String,
    pub year: i32,
    pub transmission: Option<String>,
    pub fuel_type: Option<String>,
    pub engine_capacity: Option<i32>,
    pub mileage: Option<i32>,
    pub seats: i32,
    pub doors: Option<i32>,
    pub luggage_capacity: Option<i32>,
    pub vehicle_type: String,
    pub is_luxury: bool,
    pub is_flood_free: bool,
    pub tax_active: bool,
    pub has_bpkb: bool,
    pub has_stnk: bool,
    pub description: Option<String>,
    pub rental_terms: Option<String>,
    pub city: String,
    pub address: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub area_coverage: Option<JsonValue>,
    pub photos: JsonValue,
    pub status: String,
    pub rating: Option<f64>,
    pub review_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Row helper untuk query dengan join
#[derive(sqlx::FromRow)]
pub struct VehicleWithSeller {
    // Vehicle fields
    pub id: i32,
    pub seller_id: i32,
    pub title: String,
    pub category: String,
    pub price: f64,
    pub brand: String,
    pub model: String,
    pub year: i32,
    pub transmission: Option<String>,
    pub fuel_type: Option<String>,
    pub engine_capacity: Option<i32>,
    pub mileage: Option<i32>,
    pub seats: i32,
    pub doors: Option<i32>,
    pub luggage_capacity: Option<i32>,
    pub vehicle_type: String,
    pub is_luxury: bool,
    pub is_flood_free: bool,
    pub tax_active: bool,
    pub has_bpkb: bool,
    pub has_stnk: bool,
    pub description: Option<String>,
    pub rental_terms: Option<String>,
    pub city: String,
    pub address: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub area_coverage: Option<JsonValue>,
    pub photos: JsonValue,
    pub status: String,
    pub rating: Option<f64>,
    pub review_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // Seller field
    pub seller_name: String,
}

// Request untuk create vehicle baru (seller)
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateVehicleRequest {
    #[schema(example = "Toyota Avanza 2022 - Nyaman & Irit")]
    pub title: String,
    #[schema(example = "rental")]
    pub category: String,
    #[schema(example = 350000.0)]
    pub price: f64,
    #[schema(example = "Toyota")]
    pub brand: String,
    #[schema(example = "Avanza")]
    pub model: String,
    #[schema(example = 2022)]
    pub year: i32,
    #[schema(example = "Automatic")]
    pub transmission: Option<String>,
    #[schema(example = "Bensin")]
    pub fuel_type: Option<String>,
    #[schema(example = 1500)]
    pub engine_capacity: Option<i32>,
    #[schema(example = 15000)]
    pub mileage: Option<i32>,
    #[schema(example = 7)]
    pub seats: i32,
    #[schema(example = 4)]
    pub doors: Option<i32>,
    #[schema(example = 2)]
    pub luggage_capacity: Option<i32>,
    #[schema(example = "MPV")]
    pub vehicle_type: String,
    #[schema(example = false)]
    pub is_luxury: Option<bool>,
    #[schema(example = true)]
    pub is_flood_free: Option<bool>,
    #[schema(example = true)]
    pub tax_active: Option<bool>,
    #[schema(example = true)]
    pub has_bpkb: Option<bool>,
    #[schema(example = true)]
    pub has_stnk: Option<bool>,
    #[schema(example = "Mobil keluarga yang nyaman dan irit")]
    pub description: Option<String>,
    #[schema(example = "- Wajib KTP\n- SIM A aktif\n- Booking minimal 1 hari")]
    pub rental_terms: Option<String>,
    #[schema(example = "Jakarta")]
    pub city: String,
    #[schema(example = "Jl. Sudirman No. 123, Jakarta Pusat")]
    pub address: String,
    #[schema(example = -6.208763)]
    pub latitude: Option<f64>,
    #[schema(example = 106.845599)]
    pub longitude: Option<f64>,
    pub photos: Vec<String>,
}

// Request untuk update vehicle (seller)
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateVehicleRequest {
    #[schema(example = "Toyota Avanza 2022 - Updated Title")]
    pub title: Option<String>,
    #[schema(example = 320000.0)]
    pub price: Option<f64>,
    #[schema(example = "Automatic")]
    pub transmission: Option<String>,
    #[schema(example = "Bensin")]
    pub fuel_type: Option<String>,
    #[schema(example = 1500)]
    pub engine_capacity: Option<i32>,
    #[schema(example = 16000)]
    pub mileage: Option<i32>,
    #[schema(example = 7)]
    pub seats: Option<i32>,
    #[schema(example = 4)]
    pub doors: Option<i32>,
    #[schema(example = 2)]
    pub luggage_capacity: Option<i32>,
    #[schema(example = "Updated description")]
    pub description: Option<String>,
    #[schema(example = "- Updated terms")]
    pub rental_terms: Option<String>,
    #[schema(example = "Jl. Sudirman No. 456")]
    pub address: Option<String>,
    #[schema(example = -6.208763)]
    pub latitude: Option<f64>,
    #[schema(example = 106.845599)]
    pub longitude: Option<f64>,
}

// Query parameters untuk filtering vehicles
#[derive(Debug, Deserialize, utoipa::IntoParams, ToSchema)]
pub struct VehicleFilter {
    #[schema(example = "rental")]
    pub category: Option<String>,
    #[schema(example = "Jakarta")]
    pub city: Option<String>,
    #[schema(example = "Toyota")]
    pub brand: Option<String>,
    #[schema(example = "Avanza")]
    pub model: Option<String>,
    #[schema(example = "Automatic")]
    pub transmission: Option<String>,
    #[schema(example = "Bensin")]
    pub fuel_type: Option<String>,
    #[schema(example = "MPV")]
    pub vehicle_type: Option<String>,
    #[schema(example = 200000.0)]
    pub min_price: Option<f64>,
    #[schema(example = 500000.0)]
    pub max_price: Option<f64>,
    #[schema(example = 2020)]
    pub min_year: Option<i32>,
    #[schema(example = 2024)]
    pub max_year: Option<i32>,
    #[schema(example = 5)]
    pub min_seats: Option<i32>,
    #[schema(example = false)]
    pub is_luxury: Option<bool>,
    #[schema(example = "price_asc")]
    pub sort_by: Option<String>,
    #[schema(example = 1)]
    pub page: Option<i64>,
    #[schema(example = 20)]
    pub limit: Option<i64>,
}

// Response vehicle dengan informasi seller (untuk list & detail)
#[derive(Debug, Serialize, ToSchema)]
pub struct VehicleResponse {
    pub id: i32,
    pub seller_id: i32,
    pub seller_name: String,
    pub title: String,
    pub category: String,
    pub price: f64,
    pub brand: String,
    pub model: String,
    pub year: i32,
    pub transmission: Option<String>,
    pub fuel_type: Option<String>,
    pub engine_capacity: Option<i32>,
    pub mileage: Option<i32>,
    pub seats: i32,
    pub doors: Option<i32>,
    pub luggage_capacity: Option<i32>,
    pub vehicle_type: String,
    pub is_luxury: bool,
    pub is_flood_free: bool,
    pub tax_active: bool,
    pub has_bpkb: bool,
    pub has_stnk: bool,
    pub description: Option<String>,
    pub rental_terms: Option<String>,
    pub city: String,
    pub address: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub area_coverage: Option<JsonValue>,
    pub photos: Vec<String>,
    pub status: String,
    pub rating: Option<f64>,
    pub review_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Response untuk list vehicles dengan pagination
#[derive(Debug, Serialize, ToSchema)]
pub struct VehicleListResponse {
    pub data: Vec<VehicleResponse>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
    pub total_pages: i64,
}

// Master data - City
#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct City {
    pub id: i32,
    pub name: String,
}

// Master data - Brand
#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct Brand {
    pub id: i32,
    pub name: String,
}

// Master data - Model
#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct Model {
    pub id: i32,
    pub brand_id: i32,
    pub name: String,
}
