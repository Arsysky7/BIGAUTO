use axum::{extract::{State, Query}, Json};
use serde::Deserialize;
use sqlx::PgPool;
use utoipa::ToSchema;

use crate::{
    domain::vehicle::{City, Brand, Model},
    error::AppError,
    repositories::filter_repo,
};

#[derive(Debug, Deserialize, ToSchema)]
pub struct BrandQuery {
    pub brand: Option<String>,
}

// Get list cities
#[utoipa::path(
    get,
    path = "/api/filters/cities",
    tag = "Filters",
    responses(
        (status = 200, description = "List cities", body = Vec<City>),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_cities(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<City>>, AppError> {
    let cities = filter_repo::find_all_cities(&pool).await?;
    Ok(Json(cities))
}

// Get list brands
#[utoipa::path(
    get,
    path = "/api/filters/brands",
    tag = "Filters",
    responses(
        (status = 200, description = "List brands", body = Vec<Brand>),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_brands(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<Brand>>, AppError> {
    let brands = filter_repo::find_all_brands(&pool).await?;
    Ok(Json(brands))
}

// Get list models by brand
#[utoipa::path(
    get,
    path = "/api/filters/models",
    tag = "Filters",
    params(
        ("brand" = Option<String>, Query, description = "Brand name")
    ),
    responses(
        (status = 200, description = "List models", body = Vec<Model>),
        (status = 400, description = "Brand required"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_models(
    Query(query): Query<BrandQuery>,
    State(pool): State<PgPool>,
) -> Result<Json<Vec<Model>>, AppError> {
    let brand = query
        .brand
        .ok_or_else(|| AppError::validation("Brand parameter required"))?;

    let models = filter_repo::find_models_by_brand(&pool, &brand).await?;
    Ok(Json(models))
}
