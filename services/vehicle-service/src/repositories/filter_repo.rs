use sqlx::PgPool;

use crate::{domain::vehicle::{City, Brand, Model}, error::AppError};

// Ambil list semua cities yang punya vehicles
pub async fn find_all_cities(pool: &PgPool) -> Result<Vec<City>, AppError> {
    let cities = sqlx::query_as(
        "SELECT DISTINCT
            ROW_NUMBER() OVER (ORDER BY city) as id,
            city as name
         FROM vehicles
         WHERE status = 'available'
         ORDER BY city"
    )
    .fetch_all(pool)
    .await?;

    Ok(cities)
}

// Ambil list semua brands
pub async fn find_all_brands(pool: &PgPool) -> Result<Vec<Brand>, AppError> {
    let brands = sqlx::query_as(
        "SELECT DISTINCT
            ROW_NUMBER() OVER (ORDER BY brand) as id,
            brand as name
         FROM vehicles
         WHERE status = 'available'
         ORDER BY brand"
    )
    .fetch_all(pool)
    .await?;

    Ok(brands)
}

// Ambil list models berdasarkan brand
pub async fn find_models_by_brand(pool: &PgPool, brand: &str) -> Result<Vec<Model>, AppError> {
    let models = sqlx::query_as(
        "SELECT DISTINCT
            ROW_NUMBER() OVER (ORDER BY model) as id,
            1 as brand_id,
            model as name
         FROM vehicles
         WHERE brand = $1 AND status = 'available'
         ORDER BY model"
    )
    .bind(brand)
    .fetch_all(pool)
    .await?;

    Ok(models)
}
