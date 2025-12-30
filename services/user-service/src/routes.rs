use axum::{
    http::{header, HeaderValue, Method},
    routing::{get, post, put, delete},
    Router, Json, extract::State,
};
use sqlx::PgPool;
use tower_http::cors::CorsLayer;
use utoipa::{OpenApi, Modify};
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa_swagger_ui::SwaggerUi;
use utoipa_redoc::{Redoc, Servable};
use crate::{
    handlers::{profile, favorite, rating},
    config::{AppState, HealthStatus, check_db_health},
    middleware::{auth::auth_middleware, rate_limit::rate_limit_middleware},
};

// Security scheme untuk Bearer authentication
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
        title = "Big Auto - User Service API",
        version = "0.1.0",
        description = "User Profile, Favorites, and Ratings Service\n\n## Features\n\n- 👤 User Profile Management\n- 🏪 Seller Upgrade\n- 📸 Profile Photo Upload (Cloudinary)\n- ❤️ Vehicle Favorites/Wishlist\n- ⭐ Seller Ratings & Reviews\n\n## Authentication\n\nAll endpoints require JWT token from auth-service.\nInclude token in `Authorization: Bearer {token}` header.\n",
    ),
    paths(
        // Profile endpoints
        profile::get_my_profile,
        profile::get_user_profile,
        profile::update_profile,
        profile::upgrade_to_seller,
        profile::upload_profile_photo,
        // Favorite endpoints
        favorite::get_favorites,
        favorite::add_favorite,
        favorite::remove_favorite,
        favorite::check_favorite,
        // Rating endpoints
        rating::submit_review,
        rating::get_seller_ratings,
        rating::get_seller_rating_summary,
        rating::get_my_seller_reviews,
    ),
    modifiers(&SecurityAddon),
    components(
        schemas(
            // Profile schemas
            crate::domain::user::UserProfile,
            crate::domain::user::UpdateProfileRequest,
            crate::domain::user::UpgradeToSellerRequest,
            crate::domain::user::UploadPhotoResponse,
            profile::MessageResponse,
            // Favorite schemas
            crate::domain::favorite::Favorite,
            crate::domain::favorite::AddFavoriteRequest,
            crate::domain::favorite::FavoriteWithVehicle,
            crate::domain::favorite::CheckFavoriteResponse,
            favorite::MessageResponse,
            // Rating schemas
            crate::domain::review::CreateReviewRequest,
            crate::domain::review::ReviewWithCustomer,
            crate::domain::review::SellerRatingSummary,
            crate::domain::review::RatingDistribution,
            rating::SubmitReviewResponse,
        )
    ),
    tags(
        (name = "Profile", description = "User profile management endpoints"),
        (name = "Favorites", description = "Vehicle favorites/wishlist endpoints"),
        (name = "Ratings", description = "Seller ratings and reviews endpoints")
    )
)]
struct ApiDoc;

// Health check handler
async fn health_check(State(pool): State<PgPool>) -> Json<HealthStatus> {
    let db_healthy = check_db_health(&pool).await;

    Json(HealthStatus {
        database: if db_healthy { "healthy" } else { "unhealthy" }.to_string(),
        overall: if db_healthy { "healthy" } else { "degraded" }.to_string(),
    })
}

/// Build JWT-Only CORS configuration
fn configure_cors() -> CorsLayer {
    let frontend_url = std::env::var("FRONTEND_URL")
        .expect("FRONTEND_URL environment variable harus diset");

    let allowed_methods = vec![
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::DELETE,
        Method::OPTIONS,
    ];

    let allowed_headers = vec![
        header::AUTHORIZATION,
        header::ACCEPT,
        header::CONTENT_TYPE,
    ];

    CorsLayer::new()
        .allow_origin(frontend_url.parse::<HeaderValue>().expect("Invalid FRONTEND_URL"))
        .allow_methods(allowed_methods)
        .allow_headers(allowed_headers)
        .allow_credentials(false) 
        .max_age(std::time::Duration::from_secs(86400)) 
}

/// Security headers middleware
async fn security_headers_middleware(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let mut response = next.run(request).await;

    // Add security headers
    response.headers_mut().insert("X-Content-Type-Options", HeaderValue::from_static("nosniff"));
    response.headers_mut().insert("X-Frame-Options", HeaderValue::from_static("DENY"));
    response.headers_mut().insert("X-XSS-Protection", HeaderValue::from_static("1; mode=block"));
    response.headers_mut().insert("Strict-Transport-Security", HeaderValue::from_static("max-age=31536000; includeSubDomains"));
    response.headers_mut().insert("Referrer-Policy", HeaderValue::from_static("strict-origin-when-cross-origin"));

    response
}



// Buat router dengan JWT-Only security
pub fn create_router(state: AppState) -> Router {
    // OpenAPI documentation
    let openapi = ApiDoc::openapi();

    // Complete router dengan security layers
    Router::new()
        .route("/health", get(health_check).with_state(state.db.clone()))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", openapi.clone()))
        .merge(Redoc::with_url("/redoc", openapi))
        // Security layers (JWT-Only)
        .layer(axum::middleware::from_fn(security_headers_middleware))
        .layer(configure_cors())
        // API routes dengan JWT protection
        .nest("/api", create_jwt_protected_routes(state))
}

// All API routes require JWT authentication
fn create_jwt_protected_routes(state: AppState) -> Router {
    let read_routes = Router::new()
        // READ endpoints 
        .route("/users/me", get(profile::get_my_profile))
        .route("/users/{user_id}", get(profile::get_user_profile))
        .route("/users/me/favorites", get(favorite::get_favorites))
        .route("/users/me/favorites/check/{vehicle_id}", get(favorite::check_favorite))
        .route("/sellers/{seller_id}/ratings", get(rating::get_seller_ratings))
        .route("/sellers/{seller_id}/rating-summary", get(rating::get_seller_rating_summary))
        .route("/sellers/me/reviews", get(rating::get_my_seller_reviews));

    let write_routes = Router::new()
        // WRITE endpoints 
        .route("/users/me", put(profile::update_profile))
        .route("/users/me/photo", post(profile::upload_profile_photo))
        .route("/users/me/upgrade-seller", post(profile::upgrade_to_seller))
        .route("/users/me/favorites", post(favorite::add_favorite))
        .route("/users/me/favorites/{vehicle_id}", delete(favorite::remove_favorite))
        .route("/sellers/{seller_id}/reviews", post(rating::submit_review))
        // Apply strict rate limiting to write operations
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            rate_limit_middleware
        ));

    // Combine all routes with JWT authentication
    read_routes
        .merge(write_routes)
        .with_state(state.clone())
        // Apply JWT middleware untuk semua routes
        .layer(axum::middleware::from_fn_with_state(
            state,
            auth_middleware
        ))
}