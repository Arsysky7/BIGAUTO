// Router configuration untuk Financial Service
use axum::{
    extract::State,
    http::{header, HeaderValue, Method},
    Json,
    Router,
    routing::{get, post},
};
use sqlx::PgPool;
use tower_http::cors::CorsLayer;
use utoipa::{OpenApi, Modify};
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa_swagger_ui::SwaggerUi;
use utoipa_redoc::{Redoc, Servable};
use crate::{
    config::{AppState, HealthStatus, check_db_health},
    handlers::{
        balance::{get_balance, __path_get_balance},
        transactions::{get_transactions, __path_get_transactions},
        withdrawals::{create_withdrawal, __path_create_withdrawal, list_withdrawals, __path_list_withdrawals, get_withdrawal_by_id, __path_get_withdrawal_by_id},
    },
    middleware::{auth_middleware, rate_limit_middleware},
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
        title = "Big Auto - Financial Service API",
        version = "0.1.0",
        description = "Financial Management Service\n\n## Features\n\n- 💰 Seller Balance Management\n- 💸 Withdrawal Requests\n- 📊 Transaction History\n- 💳 Commission Processing\n\n## Authentication\n\nAll endpoints require JWT token from auth-service.\nInclude token in `Authorization: Bearer {token}` header.\n",
    ),
    paths(
        health_check,
        get_balance,
        get_transactions,
        create_withdrawal,
        list_withdrawals,
        get_withdrawal_by_id,
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "Health", description = "Health check endpoints"),
        (name = "Seller Balance", description = "Seller balance management"),
        (name = "Seller Transactions", description = "Transaction history and logs"),
        (name = "Seller Withdrawals", description = "Withdrawal request management")
    )
)]
struct ApiDoc;

// Health check handler
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthStatus)
    ),
    tag = "Health"
)]
async fn health_check(State(pool): State<PgPool>) -> Json<HealthStatus> {
    let db_healthy = check_db_health(&pool).await;

    Json(HealthStatus {
        database: if db_healthy { "healthy" } else { "unhealthy" }.to_string(),
        overall: if db_healthy { "healthy" } else { "degraded" }.to_string(),
    })
}

// Build JWT-Only CORS configuration
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

// Security headers middleware
async fn security_headers_middleware(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let mut response = next.run(request).await;

    // Required security headers
    response.headers_mut().insert("X-Content-Type-Options", HeaderValue::from_static("nosniff"));
    response.headers_mut().insert("X-Frame-Options", HeaderValue::from_static("DENY"));
    response.headers_mut().insert("X-XSS-Protection", HeaderValue::from_static("1; mode=block"));
    response.headers_mut().insert("Strict-Transport-Security", HeaderValue::from_static("max-age=31536000; includeSubDomains"));
    response.headers_mut().insert("Referrer-Policy", HeaderValue::from_static("strict-origin-when-cross-origin"));

    // Content-Security-Policy 
    response.headers_mut().insert(
        "Content-Security-Policy",
        HeaderValue::from_static("default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self'; connect-src 'self'; frame-ancestors 'none'")
    );

    // Permissions-Policy 
    response.headers_mut().insert(
        "Permissions-Policy",
        HeaderValue::from_static("camera=(), microphone=(), geolocation=()")
    );

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
        .route("/seller/balance", get(get_balance))
        .route("/seller/transactions", get(get_transactions))
        .route("/seller/withdrawals", get(list_withdrawals))
        .route("/seller/withdrawals/{id}", get(get_withdrawal_by_id));

    let write_routes = Router::new()
        // WRITE endpoints 
        .route("/seller/withdrawals", post(create_withdrawal))
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