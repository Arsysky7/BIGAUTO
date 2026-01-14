use axum::{
    http::{header, HeaderValue, Method},
    routing::{get, put},
    Router, Json, extract::State,
};
use sqlx::PgPool;
use tower_http::cors::CorsLayer;
use utoipa::{OpenApi, Modify};
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa_swagger_ui::SwaggerUi;
use utoipa_redoc::{Redoc, Servable};
use crate::{
    handlers::notification,
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

// OpenAPI Documentation untuk Notification Service
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Big Auto - Notification Service API",
        version = "1.0.0",
        description = "Notification Service\n\n## Features\n\n- 📨 Get user notifications\n- ✅ Mark notification as read\n- 📬 Mark all notifications as read\n- 🔔 Get unread count\n\n## Authentication\n\nAll endpoints require JWT token from auth-service.\nInclude token in `Authorization: Bearer {token}` header.\n",
    ),
    paths(
        notification::get_notifications,
        notification::mark_as_read,
        notification::mark_all_as_read,
        notification::get_unread_count,
    ),
    modifiers(&SecurityAddon),
    components(
        schemas(
            crate::domain::notification::NotificationResponse,
            crate::domain::notification::MarkReadResponse,
            crate::domain::notification::ReadAllResponse,
            crate::domain::notification::UnreadCountResponse,
            notification::NotificationQuery,
            notification::NotificationListResponse,
        )
    ),
    tags(
        (name = "Notifications", description = "Notification management endpoints")
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

/// Buat router dengan JWT-Only security
pub fn create_router(state: AppState) -> Router {
    // OpenAPI documentation
    let openapi = ApiDoc::openapi();

    // Build route hierarchy dengan proper security layering
    let api_routes = build_api_routes_with_auth(state.clone());

    // Complete router dengan security layers
    Router::new()
        .route("/health", get(health_check).with_state(state.db.clone()))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", openapi.clone()))
        .merge(Redoc::with_url("/redoc", openapi))
        .nest("/api", api_routes)
        .layer(axum::middleware::from_fn(security_headers_middleware))
        .layer(configure_cors())
        .layer(axum::middleware::from_fn_with_state(state.clone(), rate_limit_middleware))
}

// Build API routes dengan JWT authentication
fn build_api_routes_with_auth(state: AppState) -> Router {
    // All API routes require JWT authentication
    let api_routes = Router::new()
        // Notification endpoints
        .route("/notifications", get(notification::get_notifications))
        .route("/notifications/unread-count", get(notification::get_unread_count))
        .route("/notifications/read-all", put(notification::mark_all_as_read))
        .route("/notifications/{id}/read", put(notification::mark_as_read))
        .layer(axum::middleware::from_fn_with_state(state.clone(), auth_middleware))
        .with_state(state);

    api_routes
}