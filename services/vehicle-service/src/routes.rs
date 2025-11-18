use axum::{
    routing::{get, post, put, delete},
    Router, Json, extract::State,
};
use sqlx::PgPool;
use utoipa::{OpenApi, Modify};
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa_swagger_ui::SwaggerUi;
use utoipa_redoc::{Redoc, Servable};

use crate::handlers::{vehicles, photos, filters};
use crate::config::{HealthStatus, check_db_health, AppState};

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build()
                ),
            )
        }
    }
}

// OpenAPI Documentation
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Big Auto - Vehicle Service API",
        version = "1.0.0",
        description = "Vehicle Management Service\n\n## Features\n\n- 🚗 Vehicle CRUD (Rental & Sale)\n- 📸 Photo Management\n- 🔍 Advanced Filtering\n- 📍 Master Data (Cities, Brands, Models)\n\n## Authentication\n\nSeller endpoints require JWT token.\nInclude in `Authorization: Bearer {token}` header.\n",
    ),
    paths(
        vehicles::list_vehicles,
        vehicles::get_vehicle,
        vehicles::create_vehicle,
        vehicles::update_vehicle,
        vehicles::delete_vehicle,
        photos::upload_photos,
        photos::delete_photo,
        filters::get_cities,
        filters::get_brands,
        filters::get_models,
    ),
    modifiers(&SecurityAddon),
    components(
        schemas(
            crate::domain::vehicle::VehicleResponse,
            crate::domain::vehicle::VehicleListResponse,
            crate::domain::vehicle::VehicleFilter,
            crate::domain::vehicle::CreateVehicleRequest,
            crate::domain::vehicle::UpdateVehicleRequest,
            crate::domain::vehicle::City,
            crate::domain::vehicle::Brand,
            crate::domain::vehicle::Model,
            vehicles::MessageResponse,
            filters::BrandQuery,
        )
    ),
    tags(
        (name = "Vehicles", description = "Vehicle management endpoints"),
        (name = "Photos", description = "Photo management endpoints"),
        (name = "Filters", description = "Master data filter endpoints")
    )
)]
struct ApiDoc;

// Health check endpoint
async fn health_check(State(pool): State<PgPool>) -> Json<HealthStatus> {
    let db_healthy = check_db_health(&pool).await;

    Json(HealthStatus {
        database: if db_healthy { "healthy" } else { "unhealthy" }.to_string(),
        overall: if db_healthy { "healthy" } else { "degraded" }.to_string(),
    })
}

// Buat router dengan semua endpoints
pub fn create_router(state: AppState) -> Router {
    // Log environment information
    if state.config.is_production() {
        tracing::warn!("Running in PRODUCTION mode - strict validation enabled");
    } else {
        tracing::info!("Running in DEVELOPMENT mode");
    }

    let api_routes = Router::new()
        // Vehicle routes
        .route("/vehicles", get(vehicles::list_vehicles))
        .route("/vehicles", post(vehicles::create_vehicle))
        .route("/vehicles/{id}", get(vehicles::get_vehicle))
        .route("/vehicles/{id}", put(vehicles::update_vehicle))
        .route("/vehicles/{id}", delete(vehicles::delete_vehicle))
        // Photo routes
        .route("/vehicles/{id}/photos", post(photos::upload_photos))
        .route("/vehicles/{id}/photos/{index}", delete(photos::delete_photo))
        // Filter routes
        .route("/filters/cities", get(filters::get_cities))
        .route("/filters/brands", get(filters::get_brands))
        .route("/filters/models", get(filters::get_models))
        .with_state(state.clone());

    let openapi = ApiDoc::openapi();

    Router::new()
        .route("/health", get(health_check).with_state(state.db.clone()))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", openapi.clone()))
        .merge(Redoc::with_url("/redoc", openapi))
        .nest("/api", api_routes)
}
