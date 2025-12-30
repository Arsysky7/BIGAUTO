// API Routes untuk booking-service dengan OpenAPI documentation
use axum::{
    routing::{get, post, put},
    Router, Json, extract::State,
    http::{header, HeaderValue},
};
use utoipa::{OpenApi, Modify};
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    handlers::{
        rental_handlers, testdrive_handlers, sale_handlers,
    },
    config::{AppState, HealthStatus, check_db_health},
    domain::sale::{
        CreateSaleOrderRequest, SaleOrderResponse, UpdateDocumentStatusRequest,
        SaleOrderQueryParams, UploadKtpRequest, AcceptSaleOrderRequest,
        CounterOfferRequest, CancelRequest
    },
    domain::rental::ValidateReturnRequest,
    middleware::{auth::jwt_auth_middleware, rate_limit::rate_limit_middleware},
};

// Security scheme modifier untuk Bearer authentication
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

// OpenAPI Documentation untuk booking-service
#[derive(OpenApi)]
#[openapi(
    paths(
        // Rental Bookings
        rental_handlers::create_rental_booking,
        rental_handlers::get_rental_booking,
        rental_handlers::get_customer_rental_bookings,
        rental_handlers::get_seller_rental_bookings,
        rental_handlers::update_rental_booking_status,
        rental_handlers::validate_pickup,
        rental_handlers::validate_return,
        rental_handlers::cancel_rental_booking,

        // Test Drive Bookings
        testdrive_handlers::create_testdrive_booking,
        testdrive_handlers::get_testdrive_booking,
        testdrive_handlers::get_customer_testdrive_bookings,
        testdrive_handlers::get_seller_testdrive_bookings,
        testdrive_handlers::accept_testdrive_booking,
        testdrive_handlers::reschedule_testdrive_booking,
        testdrive_handlers::choose_reschedule_slot,
        testdrive_handlers::confirm_testdrive_booking,
        testdrive_handlers::complete_testdrive_booking,
        testdrive_handlers::cancel_testdrive_booking,
        testdrive_handlers::timeout_expired_testdrives,

        // Sale Orders
        sale_handlers::create_sale_order,
        sale_handlers::get_sale_order,
        sale_handlers::get_customer_sale_orders,
        sale_handlers::get_seller_sale_orders,
        sale_handlers::confirm_sale_order,
        sale_handlers::seller_counter_offer,
        sale_handlers::accept_counter_offer,
        sale_handlers::reject_sale_order,
        sale_handlers::cancel_sale_order,
        sale_handlers::mark_sale_order_as_paid,
        sale_handlers::upload_buyer_ktp,
        sale_handlers::start_document_transfer,
        sale_handlers::update_document_status,
        sale_handlers::confirm_documents_received
    ),
    modifiers(&SecurityAddon),
    components(
        schemas(
            // Rental
            crate::domain::rental::CreateRentalRequest,
            crate::domain::rental::RentalBookingResponse,
            crate::domain::rental::ValidatePickupRequest,
            crate::domain::rental::UpdateRentalStatusRequest,
            ValidateReturnRequest,

            // Test Drive
            crate::domain::testdrive::CreateTestDriveRequest,
            crate::domain::testdrive::TestDriveBookingResponse,
            crate::domain::testdrive::RescheduleTestDriveRequest,
            crate::domain::testdrive::ChooseRescheduleSlotRequest,
            crate::domain::testdrive::ConfirmTestDriveRequest,
            crate::domain::testdrive::CompleteTestDriveRequest,

            // Sale Orders
            CreateSaleOrderRequest,
            SaleOrderResponse,
            AcceptSaleOrderRequest,
            CounterOfferRequest,
            CancelRequest,
            crate::domain::sale::RejectSaleOrderRequest,
            crate::domain::sale::StartDocumentTransferRequest,
            UpdateDocumentStatusRequest,
            UploadKtpRequest,
            SaleOrderQueryParams
        )
    ),
    tags(
        (name = "rental-bookings", description = "Manajemen booking rental mobil"),
        (name = "testdrive-bookings", description = "Manajemen booking test drive"),
        (name = "sale-orders", description = "Manajemen order pembelian mobil")
    ),
    info(
        title = "BIG AUTO - Booking Service API",
        description = "API untuk mengelola booking rental mobil, test drive, dan order pembelian mobil\n\n## Features\n\n- 🚗 Rental Vehicle Bookings\n- 🚙 Test Drive Bookings\n- 🛒 Sale Orders Management\n- 🔒 JWT-Only Authentication\n- 🌐 Redis-based Rate Limiting\n\n## Authentication\n\nAll endpoints require JWT Bearer token from auth-service.\nInclude token in `Authorization: Bearer {token}` header.\nNo CSRF tokens required - JWT-Only architecture.",
        version = "1.0.0",
        contact(
            name = "BIG AUTO Development Team",
            email = "dev@bigauto.com"
        ),
        license(
            name = "MIT",
            url = "https://opensource.org/licenses/MIT"
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


// Health check endpoint
async fn health_check(State(pool): State<sqlx::PgPool>) -> Json<HealthStatus> {
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

    // Build API routes dengan proper security layering
    let api_routes = build_api_routes_with_auth(state.clone());

    let openapi = ApiDoc::openapi();

    Router::new()
        .route("/health", get(health_check).with_state(state.db.clone()))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", openapi.clone()))
        .nest("/api", api_routes)
        .layer(axum::middleware::from_fn(security_headers_middleware))
        .layer(axum::middleware::from_fn_with_state(state.clone(), rate_limit_middleware))
}

// Build API routes dengan JWT authentication 
fn build_api_routes_with_auth(state: AppState) -> Router {
    // All API routes require JWT authentication per JWT-Only architecture
    let api_routes = Router::new()
        // Rental Bookings - All endpoints
        .route("/rentals/bookings", get(rental_handlers::get_customer_rental_bookings))
        .route("/rentals/bookings/my", get(rental_handlers::get_customer_rental_bookings))
        .route("/rentals/bookings/seller", get(rental_handlers::get_seller_rental_bookings))
        .route("/rentals/bookings/{id}", get(rental_handlers::get_rental_booking))
        .route("/rentals/bookings", post(rental_handlers::create_rental_booking))
        .route("/rentals/bookings/{id}/cancel", put(rental_handlers::cancel_rental_booking))
        .route("/rentals/bookings/{id}/status", put(rental_handlers::update_rental_booking_status))
        .route("/rentals/bookings/{id}/validate-pickup", put(rental_handlers::validate_pickup))
        .route("/rentals/bookings/{id}/validate-return", put(rental_handlers::validate_return))

        // Test Drive Bookings - All endpoints
        .route("/testdrives/bookings", get(testdrive_handlers::get_customer_testdrive_bookings))
        .route("/testdrives/bookings/my", get(testdrive_handlers::get_customer_testdrive_bookings))
        .route("/testdrives/bookings/seller", get(testdrive_handlers::get_seller_testdrive_bookings))
        .route("/testdrives/bookings/{id}", get(testdrive_handlers::get_testdrive_booking))
        .route("/testdrives/bookings", post(testdrive_handlers::create_testdrive_booking))
        .route("/testdrives/bookings/{id}/accept", put(testdrive_handlers::accept_testdrive_booking))
        .route("/testdrives/bookings/{id}/reschedule", put(testdrive_handlers::reschedule_testdrive_booking))
        .route("/testdrives/bookings/{id}/choose-slot", put(testdrive_handlers::choose_reschedule_slot))
        .route("/testdrives/bookings/{id}/confirm", put(testdrive_handlers::confirm_testdrive_booking))
        .route("/testdrives/bookings/{id}/complete", put(testdrive_handlers::complete_testdrive_booking))
        .route("/testdrives/bookings/{id}/cancel", put(testdrive_handlers::cancel_testdrive_booking))
        .route("/testdrives/timeout-expired", post(testdrive_handlers::timeout_expired_testdrives))

        // Sale Orders - All endpoints
        .route("/sales/orders/my", get(sale_handlers::get_customer_sale_orders))
        .route("/sales/orders/seller", get(sale_handlers::get_seller_sale_orders))
        .route("/sales/orders/{id}", get(sale_handlers::get_sale_order))
        .route("/sales/orders", post(sale_handlers::create_sale_order))
        .route("/sales/orders/{id}/confirm", put(sale_handlers::confirm_sale_order))
        .route("/sales/orders/{id}/counter", put(sale_handlers::seller_counter_offer))
        .route("/sales/orders/{id}/accept-counter", put(sale_handlers::accept_counter_offer))
        .route("/sales/orders/{id}/reject", put(sale_handlers::reject_sale_order))
        .route("/sales/orders/{id}/cancel", put(sale_handlers::cancel_sale_order))
        .route("/sales/orders/{id}/mark-paid", put(sale_handlers::mark_sale_order_as_paid))
        .route("/sales/orders/{id}/upload-ktp", put(sale_handlers::upload_buyer_ktp))
        .route("/sales/orders/{id}/start-documents", put(sale_handlers::start_document_transfer))
        .route("/sales/orders/{id}/update-documents", put(sale_handlers::update_document_status))
        .route("/sales/orders/{id}/confirm-documents", put(sale_handlers::confirm_documents_received))
        .layer(axum::middleware::from_fn_with_state(state.clone(), jwt_auth_middleware))
        .with_state(state);

    api_routes
}

// Security Headers Middleware untuk HTTP security 
async fn security_headers_middleware(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<impl axum::response::IntoResponse, std::convert::Infallible> {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    // Content Security Policy 
    let csp = std::env::var("CSP_POLICY").unwrap_or_else(|_| {
        "default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self'; connect-src 'self'; frame-ancestors 'none'".to_string()
    });
    headers.insert("Content-Security-Policy", csp.parse().unwrap());

    // Security headers 
    let env = std::env::var("RUST_ENV").unwrap_or_else(|_| "development".to_string());

    match env.as_str() {
        "production" => {
            headers.insert("X-Frame-Options", HeaderValue::from_static("DENY"));
            headers.insert("X-Content-Type-Options", HeaderValue::from_static("nosniff"));
            headers.insert(header::REFERRER_POLICY, HeaderValue::from_static("strict-origin-when-cross-origin"));
        }
        _ => {
            headers.insert("X-Frame-Options", HeaderValue::from_static("SAMEORIGIN"));
            headers.insert("X-Content-Type-Options", HeaderValue::from_static("nosniff"));
        }
    }

    // Common security headers
    headers.insert(header::CACHE_CONTROL, "no-store, no-cache, must-revalidate, private".parse().unwrap());
    headers.insert(header::PRAGMA, "no-cache".parse().unwrap());

    // Custom service headers
    headers.insert("X-Booking-Service-Version",
        HeaderValue::from_str(&std::env::var("APP_VERSION").unwrap_or_else(|_| "1.0.0".to_string())).unwrap());
    headers.insert("X-Service-Timestamp",
        HeaderValue::from_str(&chrono::Utc::now().to_rfc3339()).unwrap());

    // Remove server header untuk security
    headers.remove(header::SERVER);

    Ok(response)
}

