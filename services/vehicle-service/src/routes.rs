use axum::{
    routing::{get, post, put, delete},
    Router, Json, extract::State, http::StatusCode, middleware::{self, Next}, response::Response,
};
use sqlx::PgPool;
use utoipa::{OpenApi, Modify};
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa_swagger_ui::SwaggerUi;
use utoipa_redoc::{Redoc, Servable};
use std::env;

use crate::handlers::{vehicles, photos, filters};
use crate::middleware::{auth::auth_middleware, rate_limit::rate_limit_middleware};
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

// Buat router dengan JWT-Only security
pub fn create_router(state: AppState) -> Router {
    // Log environment information
    if state.config.is_production() {
        tracing::warn!("Running in PRODUCTION mode - strict validation enabled");
    } else {
        tracing::info!("Running in DEVELOPMENT mode");
    }

    // Build route hierarchy dengan proper security layering
    let api_routes = build_api_routes_with_auth(state.clone());

    let openapi = ApiDoc::openapi();

    Router::new()
        .route("/health", get(health_check).with_state(state.db.clone()))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", openapi.clone()))
        .merge(Redoc::with_url("/redoc", openapi))
        .nest("/api", api_routes)
        .layer(middleware::from_fn(security_headers_middleware))
        .layer(middleware::from_fn_with_state(state.clone(), rate_limit_middleware))
}

// Middleware untuk menambahkan security headers HTTP
async fn security_headers_middleware(request: axum::extract::Request, next: Next) -> Result<Response, StatusCode> {
    let mut response = next.run(request).await;

    let headers = response.headers_mut();

    // Content Security Policy dari environment
    let csp = env::var("CSP_POLICY").unwrap_or_else(|_| {
        "default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self'; connect-src 'self'; frame-ancestors 'none'".to_string()
    });
    headers.insert("Content-Security-Policy", csp.parse().unwrap());

    // HSTS untuk HTTPS enforcement (hanya di production)
    let environment = env::var("RUST_ENV").unwrap_or_else(|_| "development".to_string());
    if environment == "production" {
        let hsts = env::var("HSTS_MAX_AGE")
            .unwrap_or_else(|_| "31536000".to_string()); 
        headers.insert("Strict-Transport-Security", format!("max-age={}; includeSubDomains; preload", hsts).parse().unwrap());
    }

    // Protection headers lainnya
    headers.insert("X-Content-Type-Options", "nosniff".parse().unwrap());
    headers.insert("X-Frame-Options", "DENY".parse().unwrap());
    headers.insert("X-XSS-Protection", "1; mode=block".parse().unwrap());
    headers.insert("Referrer-Policy", "strict-origin-when-cross-origin".parse().unwrap());
    headers.insert("Permissions-Policy", "geolocation=(), microphone=(), camera=()".parse().unwrap());

    Ok(response)
}

// Build API routes dengan JWT authentication 
fn build_api_routes_with_auth(state: AppState) -> Router {
    // All API routes require JWT authentication
    let api_routes = Router::new()
        // Vehicles - All endpoints
        .route("/vehicles", get(vehicles::list_vehicles))
        .route("/vehicles/{id}", get(vehicles::get_vehicle))
        .route("/vehicles", post(vehicles::create_vehicle))
        .route("/vehicles/{id}", put(vehicles::update_vehicle))
        .route("/vehicles/{id}", delete(vehicles::delete_vehicle))

        // Photos - All endpoints
        .route("/vehicles/{id}/photos", post(photos::upload_photos))
        .route("/vehicles/{id}/photos/{index}", delete(photos::delete_photo))

        // Filters - All endpoints
        .route("/filters/cities", get(filters::get_cities))
        .route("/filters/brands", get(filters::get_brands))
        .route("/filters/models", get(filters::get_models))
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware))
        .with_state(state);

    api_routes
}
