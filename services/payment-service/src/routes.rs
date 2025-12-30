// API Routes untuk Payment Service dengan JWT-Only architecture

use crate::config::AppState;
use crate::handlers::payment_handler;
use crate::middleware::{auth::jwt_auth_middleware, rate_limit::rate_limit_middleware};
use axum::{
    routing::{get, post},
    Router,
    extract::Request,
    middleware::Next,
    response::Response,
    http::{Method, StatusCode, header::HeaderValue},
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use utoipa::Modify;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    trace::TraceLayer,
    timeout::TimeoutLayer,
};
use std::time::Duration;

// OpenAPI Documentation untuk Payment Service
#[derive(OpenApi)]
#[openapi(
    paths(
        payment_handler::create_payment,
        payment_handler::get_payment_by_order_id,
        payment_handler::get_user_payment_history,
        payment_handler::get_payment_details,
        payment_handler::midtrans_webhook,
        payment_handler::process_refund,
        payment_handler::get_payment_receipt,
        payment_handler::check_payment_status,
        payment_handler::cancel_payment,
        payment_handler::get_payment_methods,
        payment_handler::resend_webhook,
        payment_handler::health_check,
        payment_handler::get_service_info,
    ),
    components(
        schemas(
            crate::domain::payment::CreatePaymentRequest,
            crate::domain::payment::Payment,
            crate::domain::payment::PaymentStatus,
            crate::domain::payment::PaymentType,
            crate::domain::payment::RefundRequest,
            crate::domain::payment::WebhookResponse,
            crate::domain::payment::PaymentReceipt,
            crate::domain::payment::CustomerDetails,
            crate::domain::payment::ItemDetails,
            crate::domain::payment::MidtransChargeResponse,
            crate::domain::payment::MidtransWebhookPayload
        )
    ),
    tags(
        (name = "payment-service", description = "Payment processing service for Big Auto marketplace")
    ),
    info(
        title = "Payment Service API",
        description = "Payment processing service for Big Auto vehicle marketplace with Midtrans integration\n\n## Features\n\n- 💳 Midtrans payment gateway integration\n- 🔒 JWT-Only authentication (no CSRF required)\n- 🌐 Redis-based rate limiting\n- 📊 Payment status tracking\n- 💰 Refund processing (rental only)",
        version = "1.0.0",
        contact(
            name = "Big Auto Support",
            email = "support@bigauto.com"
        )
    ),
    servers(
        (url = "https://api.bigauto.com", description = "Production server")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub struct ApiDoc;

// Security scheme modifier untuk Bearer JWT authentication
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

// Security headers middleware
async fn security_headers_middleware(
    request: Request,
    next: Next,
) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    headers.insert("Content-Security-Policy",
        "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self'; connect-src 'self'; frame-ancestors 'none';"
            .parse().unwrap());
    headers.insert("X-Content-Type-Options", "nosniff".parse().unwrap());
    headers.insert("X-Frame-Options", "DENY".parse().unwrap());
    headers.insert("X-XSS-Protection", "1; mode=block".parse().unwrap());
    headers.insert("Referrer-Policy", "strict-origin-when-cross-origin".parse().unwrap());
    headers.insert("Permissions-Policy", "camera=(), microphone=(), geolocation=()".parse().unwrap());
    headers.insert("Strict-Transport-Security", "max-age=31536000; includeSubDomains".parse().unwrap());

    response
}

// Buat router dengan JWT-only security dan Redis rate limiting
pub async fn create_routes(state: AppState) -> Router {
    if state.config.is_production() {
        tracing::warn!("Payment Service running in PRODUCTION mode");
    } else {
        tracing::info!("Payment Service running in DEVELOPMENT mode");
    }

    // CORS configuration
    let frontend_url = std::env::var("FRONTEND_URL")
        .expect("FRONTEND_URL environment variable HARUS diisi di .env file");

    let allowed_origin = frontend_url.parse::<HeaderValue>()
        .expect("FRONTEND_URL harus valid URL format");

    let cors = CorsLayer::new()
        .allow_origin(allowed_origin)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::PATCH,
            Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::ACCEPT,
            axum::http::header::CONTENT_TYPE,
        ])
        .allow_credentials(false)
        .max_age(Duration::from_secs(86400));

    // Setup OpenAPI documentation
    let mut openapi = ApiDoc::openapi();
    SecurityAddon.modify(&mut openapi);

    // Public routes - tanpa JWT authentication
    let public_routes = Router::new()
        .route("/health", get(payment_handler::health_check))
        .route("/info", get(payment_handler::get_service_info))
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", openapi))
        .with_state(state.clone());

    // Protected API routes - dengan JWT authentication
    let protected_routes = build_api_routes(state.clone())
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            jwt_auth_middleware,
        ));

    // Combine semua routes dengan shared middleware
    public_routes
        .nest("/api", protected_routes)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT, Duration::from_secs(30)))
                .layer(cors)
        )
        .layer(axum::middleware::from_fn(security_headers_middleware))
        .layer(axum::middleware::from_fn_with_state(
            Arc::new(state.rate_limiter.clone()),
            rate_limit_middleware,
        ))
}

// Build API routes dengan JWT authentication
fn build_api_routes(state: AppState) -> Router {
    Router::new()
        // ===== Payment Operations =====
        .route("/payments", post(payment_handler::create_payment))
        .route("/payments/{order_id}", get(payment_handler::get_payment_by_order_id).post(payment_handler::cancel_payment))
        .route("/payments/details/{payment_id}", get(payment_handler::get_payment_details))
        .route("/payments/status/{order_id}", get(payment_handler::check_payment_status))
        .route("/payments/user/{user_id}", get(payment_handler::get_user_payment_history))
        .route("/payments/receipt/{order_id}", get(payment_handler::get_payment_receipt))

        // ===== Refund Operations =====
        .route("/refunds", post(payment_handler::process_refund))

        // ===== Payment Methods =====
        .route("/payment-methods", get(payment_handler::get_payment_methods))

        // ===== Webhook (External - Midtrans) =====
        .route("/webhooks/midtrans", post(payment_handler::midtrans_webhook))
        .route("/webhooks/resend", post(payment_handler::resend_webhook))
        .with_state(state)
}