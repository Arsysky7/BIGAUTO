// API Routes untuk booking-service dengan OpenAPI documentation
use axum::{
    routing::{get, post, put},
    Router,
};
use utoipa::{OpenApi, Modify};
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    handlers::{
        rental_handlers, testdrive_handlers, sale_handlers,
    },
    AppState,
    domain::sale::{
        CreateSaleOrderRequest, SaleOrderResponse, UpdateDocumentStatusRequest,
        SaleOrderQueryParams, UploadKtpRequest, AcceptSaleOrderRequest,
        CounterOfferRequest, CancelRequest
    },
    domain::rental::ValidateReturnRequest,
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
        description = "API untuk mengelola booking rental mobil, test drive, dan order pembelian mobil",
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
        (url = "http://localhost:3004", description = "Development server"),
        (url = "https://api.bigauto.com", description = "Production server")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub struct ApiDoc;


// Build router dengan semua routes
pub fn create_router() -> Router<AppState> {
    Router::new()
        // Rental Bookings Routes
        .route("/api/rentals/bookings", post(rental_handlers::create_rental_booking))
        .route("/api/rentals/bookings", get(rental_handlers::get_seller_rental_bookings))
        .route("/api/rentals/bookings/my", get(rental_handlers::get_customer_rental_bookings))
        .route("/api/rentals/bookings/{id}", get(rental_handlers::get_rental_booking))
        .route("/api/rentals/bookings/{id}/cancel", put(rental_handlers::cancel_rental_booking))
        .route("/api/rentals/bookings/{id}/status", put(rental_handlers::update_rental_booking_status))
        .route("/api/rentals/bookings/{id}/validate-pickup", put(rental_handlers::validate_pickup))
        .route("/api/rentals/bookings/{id}/validate-return", put(rental_handlers::validate_return))

        // Test Drive Bookings Routes
        .route("/api/testdrives/bookings", post(testdrive_handlers::create_testdrive_booking))
        .route("/api/testdrives/bookings", get(testdrive_handlers::get_seller_testdrive_bookings))
        .route("/api/testdrives/bookings/my", get(testdrive_handlers::get_customer_testdrive_bookings))
        .route("/api/testdrives/bookings/{id}", get(testdrive_handlers::get_testdrive_booking))
        .route("/api/testdrives/bookings/{id}/accept", put(testdrive_handlers::accept_testdrive_booking))
        .route("/api/testdrives/bookings/{id}/reschedule", put(testdrive_handlers::reschedule_testdrive_booking))
        .route("/api/testdrives/bookings/{id}/choose-slot", put(testdrive_handlers::choose_reschedule_slot))
        .route("/api/testdrives/bookings/{id}/confirm", put(testdrive_handlers::confirm_testdrive_booking))
        .route("/api/testdrives/bookings/{id}/complete", put(testdrive_handlers::complete_testdrive_booking))
        .route("/api/testdrives/bookings/{id}/cancel", put(testdrive_handlers::cancel_testdrive_booking))
        .route("/api/testdrives/timeout-expired", post(testdrive_handlers::timeout_expired_testdrives))

        // Sale Orders Routes
        .route("/api/sales/orders", post(sale_handlers::create_sale_order))
        .route("/api/sales/orders/my", get(sale_handlers::get_customer_sale_orders))
        .route("/api/sales/orders/seller", get(sale_handlers::get_seller_sale_orders))
        .route("/api/sales/orders/{id}", get(sale_handlers::get_sale_order))
        .route("/api/sales/orders/{id}/confirm", put(sale_handlers::confirm_sale_order))
        .route("/api/sales/orders/{id}/counter", put(sale_handlers::seller_counter_offer))
        .route("/api/sales/orders/{id}/accept-counter", put(sale_handlers::accept_counter_offer))
        .route("/api/sales/orders/{id}/reject", put(sale_handlers::reject_sale_order))
        .route("/api/sales/orders/{id}/cancel", put(sale_handlers::cancel_sale_order))
        .route("/api/sales/orders/{id}/mark-paid", put(sale_handlers::mark_sale_order_as_paid))
        .route("/api/sales/orders/{id}/upload-ktp", put(sale_handlers::upload_buyer_ktp))
        .route("/api/sales/orders/{id}/start-documents", put(sale_handlers::start_document_transfer))
        .route("/api/sales/orders/{id}/update-documents", put(sale_handlers::update_document_status))
        .route("/api/sales/orders/{id}/confirm-documents", put(sale_handlers::confirm_documents_received))

        // Swagger UI documentation
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
}


