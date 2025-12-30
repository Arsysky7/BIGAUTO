use crate::domain::payment::{
    CreatePaymentRequest, Payment, PaymentStatus, PaymentType,
    RefundRequest, WebhookResponse, PaymentReceipt
};
use crate::handlers::midtrans_service::MidtransService;
use crate::error::AppError;
use axum::{
    extract::{Path, State},
    response::Json,
    http::HeaderMap,
};
use serde_json::{json, Value};
use chrono::Utc;
use crate::middleware::auth::AuthUser;
use sqlx::PgPool;
use utoipa;


/// Create new payment with Midtrans integration
#[utoipa::path(
    post,
    path = "/api/payments",
    tag = "Payment Service",
    summary = "Create new payment",
    description = "Create a new payment for rental booking or sale order with Midtrans integration",
    request_body = CreatePaymentRequest,
    responses(
        (status = 200, description = "Payment created successfully", body = serde_json::Value),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn create_payment(
    auth: AuthUser,
    State(app_state): State<crate::config::AppState>,
    Json(request): Json<CreatePaymentRequest>,
) -> Result<Json<Value>, AppError> {
    // security validasi
    validate_booking_order_ownership(&auth, &request, &app_state.db).await?;

    // Validasi request awal
    validate_payment_request(&request)?;

    // Cek duplikasi payment menggunakan repository
    let payment_exists = match request.payment_for_type {
        PaymentType::Rental => {
            if let Some(booking_id) = request.rental_booking_id {
                app_state.payment_repository.exists_for_rental_booking(booking_id).await?
            } else {
                false
            }
        }
        PaymentType::Sale => {
            if let Some(sale_order_id) = request.sale_order_id {
                app_state.payment_repository.exists_for_sale_order(sale_order_id).await?
            } else {
                false
            }
        }
    };

    if payment_exists {
        return Ok(Json(json!({
            "success": true,
            "message": "Payment already exists for this booking/order"
        })));
    }

    // Generate order ID unik berdasarkan tipe
    let order_id = Payment::generate_order_id(request.payment_for_type.clone());
    let expiry_time = Payment::generate_expiry_time(request.payment_for_type.clone());

    // Create Midtrans service
    let midtrans_service = MidtransService::new(
        app_state.config.midtrans_server_key.clone(),
        app_state.config.midtrans_client_key.clone(),
        app_state.config.midtrans_api_url.clone(),
    );

    // Proses charge ke Midtrans
    let midtrans_response = midtrans_service
        .charge_payment(&request, order_id.clone())
        .await
        .map_err(|e| {
            tracing::error!("Midtrans charge failed: {} - {}", order_id, e);
            e
        })?;

    // Simpan payment ke database
    let payment = app_state.payment_repository.create_payment(&request, &order_id, &midtrans_response, expiry_time).await?;

    // Generate instruksi pembayaran
    let instructions = if let Some(vas) = &midtrans_response.va_numbers {
        if let Some(first_va) = vas.first() {
            midtrans_service.get_payment_instructions(&first_va.bank, &first_va.va_number)
        } else {
            "Payment instructions will be provided by payment provider.".to_string()
        }
    } else {
        "Payment instructions will be provided by payment provider.".to_string()
    };

    // Log untuk audit
    log_payment_created(&order_id, &request);

    Ok(Json(json!({
        "success": true,
        "message": "Payment created successfully",
        "data": {
            "payment_id": payment.id,
            "order_id": order_id,
            "transaction_id": midtrans_response.transaction_id,
            "payment_type": request.payment_for_type,
            "gross_amount": request.gross_amount,
            "status": payment.status,
            "payment_method": midtrans_response.payment_type,
            "va_number": midtrans_response.va_numbers,
            "instructions": instructions,
            "expired_at": expiry_time
        }
    })))
}

/// Get payment details by order ID
#[utoipa::path(
    get,
    path = "/api/payments/{order_id}",
    tag = "Payment Service",
    summary = "Get payment by order ID",
    description = "Retrieve payment details using the unique order identifier",
    params(
        ("order_id" = String, Path, description = "Unique order identifier")
    ),
    responses(
        (status = 200, description = "Payment details retrieved successfully", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Payment not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_payment_by_order_id(
    auth: AuthUser,
    State(app_state): State<crate::config::AppState>,
    Path(order_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let payment = app_state.payment_repository.find_by_order_id(&order_id)
        .await?
        .ok_or_else(|| AppError::not_found("Payment not found"))?;

    // Security: Validate user can access this payment
    validate_payment_ownership(&auth, &payment, &app_state.db).await?;

    tracing::info!("Payment retrieved: {} by user: {}", order_id, auth.user_id);

    Ok(Json(json!({
        "success": true,
        "data": format_payment_response(&payment)
    })))
}

/// Get user payment history
#[utoipa::path(
    get,
    path = "/api/payments/user/{user_id}",
    tag = "Payment Service",
    summary = "Get user payment history",
    description = "Retrieve payment history for a specific user",
    params(
        ("user_id" = i32, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "Payment history retrieved successfully", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_user_payment_history(
    auth: AuthUser,
    State(app_state): State<crate::config::AppState>,
    Path(user_id): Path<i32>,
) -> Result<Json<Value>, AppError> {

    if auth.user_id != user_id {
        return Err(AppError::forbidden("Access denied: You can only view your own payment history"));
    }

    let payments = app_state.payment_repository.find_by_user_id(user_id).await?;

    tracing::info!("User payment history: {} - {} records", user_id, payments.len());

    Ok(Json(json!({
        "success": true,
        "count": payments.len(),
        "data": payments.iter().map(|p| format_payment_summary(p)).collect::<Vec<_>>()
    })))
}

/// Get payment details by payment ID
#[utoipa::path(
    get,
    path = "/api/payments/details/{payment_id}",
    tag = "Payment Service",
    summary = "Get payment details by payment ID",
    description = "Retrieve payment details using the payment database ID",
    params(
        ("payment_id" = i32, Path, description = "Payment database ID")
    ),
    responses(
        (status = 200, description = "Payment details retrieved successfully", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Payment not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_payment_details(
    auth: AuthUser,
    State(app_state): State<crate::config::AppState>,
    Path(payment_id): Path<i32>,
) -> Result<Json<Value>, AppError> {
    let payment = app_state.payment_repository.find_by_id(payment_id).await?
        .ok_or_else(|| AppError::not_found("Payment not found"))?;

    validate_payment_ownership(&auth, &payment, &app_state.db).await?;

    tracing::info!("Payment details: {} by user: {}", payment_id, auth.user_id);

    Ok(Json(json!({
        "success": true,
        "data": format_payment_response(&payment)
    })))
}

/// Handle Midtrans webhook notifications
#[utoipa::path(
    post,
    path = "/api/webhooks/midtrans",
    tag = "Payment Service",
    summary = "Handle Midtrans webhook",
    description = "Process payment status updates from Midtrans via webhook",
    responses(
        (status = 200, description = "Webhook processed successfully", body = WebhookResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn midtrans_webhook(
    State(app_state): State<crate::config::AppState>,
    headers: axum::http::HeaderMap,
    body: String,
) -> Result<Json<WebhookResponse>, AppError> {
    // Extract dan validasi signature
    let signature = extract_signature(&headers)?;

    // Parse payload
    let midtrans_service = MidtransService::new(
        app_state.config.midtrans_server_key.clone(),
        app_state.config.midtrans_client_key.clone(),
        app_state.config.midtrans_api_url.clone(),
    );

    let webhook_payload = midtrans_service
        .parse_webhook_payload(&body)
        .map_err(|e| {
            tracing::error!("Webhook parse failed: {}", e);
            e
        })?;

    // Verify signature
    let is_valid = midtrans_service.verify_webhook_signature(&body, &signature, &webhook_payload.order_id);
    if !is_valid {
        return Err(AppError::unauthorized("Invalid webhook signature"));
    }

    // Find payment
    let payment = app_state.payment_repository.find_by_order_id(&webhook_payload.order_id)
        .await?
        .ok_or_else(|| AppError::not_found("Payment not found"))?;

    // Update status payment dengan transaction log
    let new_status = midtrans_service.convert_status(&webhook_payload.transaction_status);
    app_state.payment_repository.update_status_with_transaction_log(
        payment.id,
        &new_status,
        Some(&webhook_payload.transaction_id),
        &webhook_payload,
    ).await?;

    // Log webhook processing
    tracing::info!(
        "Webhook processed: {} - {} -> {}",
        webhook_payload.order_id,
        webhook_payload.transaction_status,
        new_status
    );

    Ok(Json(WebhookResponse {
        success: true,
        message: "Webhook processed successfully".to_string(),
        order_id: webhook_payload.order_id,
        status: new_status,
        transaction_id: webhook_payload.transaction_id,
    }))
}

/// Process refund request
#[utoipa::path(
    post,
    path = "/api/refunds",
    tag = "Payment Service",
    summary = "Process refund",
    description = "Process refund for successful rental payments",
    request_body = RefundRequest,
    responses(
        (status = 200, description = "Refund processed successfully", body = serde_json::Value),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn process_refund(
    auth: AuthUser,
    State(app_state): State<crate::config::AppState>,
    Json(request): Json<RefundRequest>,
) -> Result<Json<Value>, AppError> {
    // Validasi dan retrieve payment
    let payment = app_state.payment_repository.find_by_order_id(&request.order_id)
        .await?
        .ok_or_else(|| AppError::not_found("Payment not found"))?;

    validate_payment_ownership(&auth, &payment, &app_state.db).await?;

    // Validasi status
    if payment.status != PaymentStatus::Success {
        return Err(AppError::refund("Refund only available for successful payments"));
    }

    if payment.is_expired() {
        return Err(AppError::refund("Expired payments cannot be refunded"));
    }

    if matches!(payment.payment_for_type, PaymentType::Sale) {
        return Err(AppError::payment("Sale payments are final and cannot be refunded"));
    }

    // Validasi business rules
    check_refund_eligibility(&payment, &request)?;

    // Generate refund ID
    let refund_id = generate_refund_id(&payment.order_id);

    // Proses refund menggunakan repository
    app_state.payment_repository.process_refund(
        payment.id,
        &refund_id,
        request.refund_amount,
        &request.reason,
    ).await?;

    // Log refund
    tracing::info!(
        "Refund processed: {} - {} ({} IDR)",
        payment.order_id, refund_id, request.refund_amount
    );

    Ok(Json(json!({
        "success": true,
        "message": "Refund processed successfully",
        "data": {
            "refund_id": refund_id,
            "order_id": payment.order_id,
            "refund_amount": request.refund_amount,
            "status": "processing"
        }
    })))
}

/// Get payment receipt
#[utoipa::path(
    get,
    path = "/api/receipts/{order_id}",
    tag = "Payment Service",
    summary = "Get payment receipt",
    description = "Generate receipt for successful payment",
    params(
        ("order_id" = String, Path, description = "Unique order identifier")
    ),
    responses(
        (status = 200, description = "Receipt generated successfully", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Payment not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_payment_receipt(
    auth: AuthUser,
    State(app_state): State<crate::config::AppState>,
    Path(order_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let payment = app_state.payment_repository.find_by_order_id(&order_id)
        .await?
        .ok_or_else(|| AppError::not_found("Payment not found"))?;

    // Security: Validate user can access this payment
    validate_payment_ownership(&auth, &payment, &app_state.db).await?;

    // Validasi status
    if payment.status != PaymentStatus::Success {
        return Err(AppError::bad_request("Payment receipt only available for successful payments"));
    }

    let receipt = PaymentReceipt::from_payment(&payment);

    // Update receipt path di database untuk tracking
    let receipt_path = format!("/receipts/{}.pdf", receipt.receipt_id);
    app_state.payment_repository.update_receipt_path(payment.id, receipt_path.clone()).await?;

    tracing::info!("Receipt generated: {} - {} (path: {})", order_id, receipt.receipt_id, receipt_path);

    Ok(Json(json!({
        "success": true,
        "data": receipt
    })))
}

/// Check payment status
#[utoipa::path(
    get,
    path = "/api/payments/status/{order_id}",
    tag = "Payment Service",
    summary = "Check payment status",
    description = "Check real-time payment status",
    params(
        ("order_id" = String, Path, description = "Unique order identifier")
    ),
    responses(
        (status = 200, description = "Payment status retrieved successfully", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Payment not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn check_payment_status(
    auth: AuthUser,
    State(app_state): State<crate::config::AppState>,
    Path(order_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let payment = app_state.payment_repository.find_by_order_id(&order_id)
        .await?
        .ok_or_else(|| AppError::not_found("Payment not found"))?;

    validate_payment_ownership(&auth, &payment, &app_state.db).await?;

    tracing::info!("Payment status checked: {} - {} by user: {}", order_id, payment.status, auth.user_id);

    Ok(Json(json!({
        "success": true,
        "data": {
            "order_id": payment.order_id,
            "status": payment.status,
            "transaction_id": payment.transaction_id,
            "is_expired": payment.is_expired(),
            "expired_at": payment.expired_at,
            "can_be_refunded": payment.can_be_refunded(),
            "payment_type": payment.payment_type
        }
    })))
}

/// Cancel pending payment
#[utoipa::path(
    post,
    path = "/api/payments/cancel",
    tag = "Payment Service",
    summary = "Cancel payment",
    description = "Cancel pending payment",
    params(
        ("order_id" = String, Path, description = "Unique order identifier")
    ),
    responses(
        (status = 200, description = "Payment cancelled successfully", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Payment not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn cancel_payment(
    auth: AuthUser,
    State(app_state): State<crate::config::AppState>,
    Path(order_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let payment = app_state.payment_repository.find_by_order_id(&order_id)
        .await?
        .ok_or_else(|| AppError::not_found("Payment not found"))?;

    validate_payment_ownership(&auth, &payment, &app_state.db).await?;

    // Validasi status
    if payment.status != PaymentStatus::Pending {
        return Err(AppError::bad_request("Only pending payments can be cancelled"));
    }

    // Update status menggunakan repository
    app_state.payment_repository.update_status(
        payment.id,
        PaymentStatus::Failed,
        None,
    ).await?;

    tracing::info!("Payment cancelled: {}", order_id);

    Ok(Json(json!({
        "success": true,
        "message": "Payment cancelled successfully",
        "order_id": order_id
    })))
}

/// Health check endpoint
#[utoipa::path(
    get,
    path = "/health",
    tag = "Payment Service",
    summary = "Health check",
    description = "Check if payment service is running",
    responses(
        (status = 200, description = "Service is healthy", body = serde_json::Value)
    )
)]
pub async fn health_check() -> Result<Json<Value>, AppError> {
    Ok(Json(json!({
        "status": "healthy",
        "service": "payment-service",
        "timestamp": Utc::now(),
        "version": "1.0.0",
    })))
}

/// Get service information
#[utoipa::path(
    get,
    path = "/info",
    tag = "Payment Service",
    summary = "Get service information",
    description = "Get payment service details and configuration",
    responses(
        (status = 200, description = "Service information retrieved successfully", body = serde_json::Value),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_service_info(
    State(config): State<crate::config::AppConfig>,
) -> Result<Json<Value>, AppError> {
    let midtrans_service = MidtransService::new(
        config.midtrans_server_key.clone(),
        config.midtrans_client_key.clone(),
        config.midtrans_api_url.clone(),
    );

    let is_prod = midtrans_service.is_production();
    tracing::info!("Payment service info requested - Production: {}", is_prod);

    Ok(Json(json!({
        "service": "payment-service",
        "version": "1.0.0",
        "environment": midtrans_service.get_environment_info(),
        "is_production": is_prod,
        "supported_payment_methods": ["bca", "bni", "mandiri", "bri", "permata"],
        "supported_payment_types": ["rental", "sale"],
        "features": [
            "Virtual Account payments",
            "Real-time payment status",
            "HMAC SHA512 webhook verification",
            "Refund processing (rental only)",
            "Payment receipts",
            "Polymorphic payment support"
        ],
        "timestamp": Utc::now()
    })))
}

// Helper Functions untuk Payment Handlers

// Validasi payment request
fn validate_payment_request(request: &CreatePaymentRequest) -> crate::error::AppResult<()> {
    if request.gross_amount <= 0 {
        return Err(AppError::validation("Gross amount must be greater than 0"));
    }

    // Validasi payment type
    match request.payment_for_type {
        PaymentType::Rental => {
            if request.rental_booking_id.is_none() {
                return Err(AppError::validation("Rental booking ID is required for rental payments"));
            }
        },
        PaymentType::Sale => {
            if request.sale_order_id.is_none() {
                return Err(AppError::validation("Sale order ID is required for sale payments"));
            }
        }
    }

    Ok(())
}

// Extract webhook signature dari headers
fn extract_signature(headers: &HeaderMap) -> crate::error::AppResult<String> {
    headers
        .get("x-callback-token")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::unauthorized("Missing signature header"))
}

// Log payment creation untuk audit
fn log_payment_created(order_id: &str, request: &CreatePaymentRequest) {
    tracing::info!(
        "Payment created: order_id={}, type={}, amount={}, booking_id={:?}, sale_id={:?}",
        order_id,
        request.payment_for_type,
        request.gross_amount,
        request.rental_booking_id,
        request.sale_order_id
    );
}

// Format payment response untuk API
fn format_payment_response(payment: &Payment) -> Value {
    json!({
        "id": payment.id,
        "order_id": payment.order_id,
        "transaction_id": payment.transaction_id,
        "payment_type": payment.payment_type,
        "payment_for_type": payment.payment_for_type,
        "gross_amount": payment.gross_amount,
        "status": payment.status,
        "va_number": payment.va_number,
        "bank": payment.bank,
        "refund_amount": payment.refund_amount,
        "refund_reason": payment.refund_reason,
        "paid_at": payment.paid_at,
        "expired_at": payment.expired_at,
        "refunded_at": payment.refunded_at,
        "receipt_pdf_path": payment.receipt_pdf_path,
        "created_at": payment.created_at,
        "updated_at": payment.updated_at,
        "is_expired": payment.is_expired(),
        "can_be_refunded": payment.can_be_refunded()
    })
}

// Format payment summary untuk list
fn format_payment_summary(payment: &Payment) -> Value {
    json!({
        "id": payment.id,
        "order_id": payment.order_id,
        "payment_type": payment.payment_type,
        "payment_for_type": payment.payment_for_type,
        "gross_amount": payment.gross_amount,
        "status": payment.status,
        "created_at": payment.created_at,
        "expired_at": payment.expired_at
    })
}

// Check refund eligibility
fn check_refund_eligibility(payment: &Payment, request: &RefundRequest) -> crate::error::AppResult<()> {
    // Validasi amount
    if request.refund_amount <= 0 {
        return Err(AppError::validation("Refund amount must be greater than 0"));
    }

    if request.refund_amount > payment.gross_amount {
        return Err(AppError::validation("Refund amount cannot exceed gross amount"));
    }

    // Validasi reason
    if request.reason.trim().is_empty() {
        return Err(AppError::validation("Refund reason is required"));
    }

    Ok(())
}

// Generate unique refund ID
fn generate_refund_id(order_id: &str) -> String {
    format!("REF-{}-{}", order_id, chrono::Utc::now().timestamp())
}

// Security: Validate user can create payment for booking/order
async fn validate_booking_order_ownership(
    auth: &AuthUser,
    request: &CreatePaymentRequest,
    pool: &PgPool,
) -> Result<(), AppError> {
    let user_id = auth.user_id;

    match request.payment_for_type {
        PaymentType::Rental => {
            if let Some(booking_id) = request.rental_booking_id {
                let result = sqlx::query!(
                    "SELECT customer_id FROM rental_bookings WHERE id = $1",
                    booking_id
                )
                .fetch_optional(pool)
                .await?
                .ok_or_else(|| AppError::not_found("Rental booking not found"))?;

                // Only customers can create payments for rentals
                if result.customer_id != user_id {
                    return Err(AppError::forbidden("Access denied: Only customers can create rental payments"));
                }
            } else {
                return Err(AppError::validation("Rental booking ID is required for rental payments"));
            }
        },
        PaymentType::Sale => {
            if let Some(sale_order_id) = request.sale_order_id {
                let result = sqlx::query!(
                    "SELECT buyer_id FROM sale_orders WHERE id = $1",
                    sale_order_id
                )
                .fetch_optional(pool)
                .await?
                .ok_or_else(|| AppError::not_found("Sale order not found"))?;

                // Only customers can create payments for sales
                if result.buyer_id != user_id {
                    return Err(AppError::forbidden("Access denied: Only customers can create sale payments"));
                }
            } else {
                return Err(AppError::validation("Sale order ID is required for sale payments"));
            }
        }
    }

    Ok(())
}

// Security: Validate user access to payment
async fn validate_payment_ownership(
    auth: &AuthUser,
    payment: &Payment,
    pool: &PgPool
) -> Result<(), AppError> {
    let user_id = auth.user_id;

    match payment.payment_for_type {
        PaymentType::Rental => {
            if let Some(booking_id) = payment.rental_booking_id {
                let booking = sqlx::query!(
                    "SELECT customer_id, seller_id FROM rental_bookings WHERE id = $1",
                    booking_id
                )
                .fetch_optional(pool)
                .await?
                .ok_or_else(|| AppError::not_found("Associated booking not found"))?;

                // User must be either customer or seller of the booking
                if booking.customer_id != user_id && booking.seller_id != user_id {
                    return Err(AppError::forbidden("Access denied: Not involved in this payment"));
                }
            } else {
                return Err(AppError::bad_request("Invalid payment: Missing rental booking reference"));
            }
        },
        PaymentType::Sale => {
            if let Some(sale_order_id) = payment.sale_order_id {
                let sale_order = sqlx::query!(
                    "SELECT buyer_id, seller_id FROM sale_orders WHERE id = $1",
                    sale_order_id
                )
                .fetch_optional(pool)
                .await?
                .ok_or_else(|| AppError::not_found("Associated sale order not found"))?;

                // User must be either buyer or seller of the sale order
                if sale_order.buyer_id != user_id && sale_order.seller_id != user_id {
                    return Err(AppError::forbidden("Access denied: Not involved in this payment"));
                }
            } else {
                return Err(AppError::bad_request("Invalid payment: Missing sale order reference"));
            }
        }
    }

    Ok(())
}

/// Get available payment methods from Midtrans configuration
#[utoipa::path(
    get,
    path = "/api/payment-methods",
    tag = "Payment Service",
    summary = "Get available payment methods",
    description = "Retrieve list of supported payment methods with details",
    responses(
        (status = 200, description = "Payment methods retrieved successfully", body = serde_json::Value),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_payment_methods() -> impl axum::response::IntoResponse {
    tracing::info!("📋 Fetching available payment methods");

    let payment_methods = vec![
        serde_json::json!({
            "code": "bca_va",
            "name": "BCA Virtual Account",
            "type": "bank_transfer",
            "icon": "https://upload.wikimedia.org/wikipedia/id/thumb/5/55/Bank_Central_Asia.svg/200px-Bank_Central_Asia.svg.png",
            "description": "Transfer melalui ATM, Mobile Banking, atau Internet Banking BCA",
            "fee_type": "fixed",
            "fee_amount": 0,
            "min_amount": 10000,
            "max_amount": 100000000,
            "available": true
        }),
        serde_json::json!({
            "code": "bni_va",
            "name": "BNI Virtual Account",
            "type": "bank_transfer",
            "icon": "https://upload.wikimedia.org/wikipedia/id/thumb/e/e3/Logo_BNI.svg/200px-Logo_BNI.svg.png",
            "description": "Transfer melalui ATM, Mobile Banking, atau Internet Banking BNI",
            "fee_type": "fixed",
            "fee_amount": 0,
            "min_amount": 10000,
            "max_amount": 100000000,
            "available": true
        }),
        serde_json::json!({
            "code": "mandiri_va",
            "name": "Mandiri Virtual Account",
            "type": "bank_transfer",
            "icon": "https://upload.wikimedia.org/wikipedia/id/thumb/5/55/Bank_Mandiri_logo.svg/200px-Bank_Mandiri_logo.svg.png",
            "description": "Transfer melalui ATM, Mobile Banking, atau Internet Banking Mandiri",
            "fee_type": "fixed",
            "fee_amount": 0,
            "min_amount": 10000,
            "max_amount": 100000000,
            "available": true
        }),
        serde_json::json!({
            "code": "bri_va",
            "name": "BRI Virtual Account",
            "type": "bank_transfer",
            "icon": "https://upload.wikimedia.org/wikipedia/id/thumb/6/68/Bank_Rakyat_Indonesia_logo.svg/200px-Bank_Rakyat_Indonesia_logo.svg.png",
            "description": "Transfer melalui ATM, Mobile Banking, atau Internet Banking BRI",
            "fee_type": "fixed",
            "fee_amount": 0,
            "min_amount": 10000,
            "max_amount": 100000000,
            "available": true
        }),
        serde_json::json!({
            "code": "permata_va",
            "name": "Permata Virtual Account",
            "type": "bank_transfer",
            "icon": "https://upload.wikimedia.org/wikipedia/id/thumb/a/a8/PermataBank.svg/200px-PermataBank.svg.png",
            "description": "Transfer melalui ATM, Mobile Banking, atau Internet Banking Permata",
            "fee_type": "fixed",
            "fee_amount": 0,
            "min_amount": 10000,
            "max_amount": 100000000,
            "available": true
        })
    ];

    let response = serde_json::json!({
        "success": true,
        "data": payment_methods,
        "total": payment_methods.len(),
        "timestamp": Utc::now().to_rfc3339()
    });

    axum::Json(response)
}

/// Resend webhook for missed payments
#[utoipa::path(
    post,
    path = "/api/webhooks/resend/{payment_id}",
    tag = "Payment Service",
    summary = "Resend webhook",
    description = "Manually trigger webhook resend for missed payment notifications",
    params(
        ("payment_id" = i32, Path, description = "Payment database ID")
    ),
    responses(
        (status = 200, description = "Webhook resent successfully", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Payment not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn resend_webhook(
    auth: AuthUser,
    Path(payment_id): Path<i32>,
    State(app_state): State<crate::config::AppState>,
) -> Result<Json<Value>, AppError> {
    tracing::info!("🔄 Manual webhook resend requested for payment_id: {}", payment_id);

    // Validate payment ownership
    let payment = app_state.payment_repository.find_by_id(payment_id).await?
        .ok_or_else(|| AppError::not_found("Payment not found"))?;

    validate_payment_ownership(&auth, &payment, &app_state.db).await?;

    // Cek apakah payment masih relevan untuk webhook resend
    if payment.status == PaymentStatus::Success {
        return Ok(Json(json!({
            "success": false,
            "message": "Payment already successful",
            "payment_id": payment_id
        })));
    }

    if payment.status == PaymentStatus::Failed {
        return Ok(Json(json!({
            "success": false,
            "message": "Payment already cancelled",
            "payment_id": payment_id
        })));
    }

    // Log manual webhook request untuk security auditing
    sqlx::query!(
        "INSERT INTO audit_logs (user_id, action, entity_type, entity_id, old_values, new_values, request_id, service_name, endpoint, http_method)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        auth.user_id,
        "MANUAL_WEBHOOK_RESEND",
        "payment",
        payment_id,
        json!({"status": payment.status.to_string()}),
        json!({"triggered_by": auth.email}),
        format!("webhook-resend-{}", payment_id),
        "payment-service",
        format!("/webhook-resend/{}", payment_id),
        "POST"
    )
    .execute(&app_state.db)
    .await?;

    // Trigger Midtrans status check dengan proper constructor
    let midtrans_service = MidtransService::new(
        app_state.config.midtrans_server_key.clone(),
        app_state.config.midtrans_client_key.clone(),
        app_state.config.midtrans_api_url.clone(),
    );
    let transaction_id = payment.transaction_id.as_ref()
        .ok_or_else(|| AppError::bad_request("Payment missing transaction ID"))?;

    match midtrans_service.check_transaction_status(transaction_id).await {
        Ok(transaction_response) => {
            // Extract transaction status dari response
            let transaction_status = transaction_response
                .get("transaction_status")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            // Update payment status berdasarkan response dari Midtrans
            let new_status = match transaction_status {
                "settlement" => PaymentStatus::Success,
                "cancel" | "expire" | "deny" => PaymentStatus::Failed,
                "pending" => PaymentStatus::Pending,
                _ => payment.status
            };

            // Update status di database jika berubah
            if new_status != payment.status {
                app_state.payment_repository.update_status(payment_id, new_status, None).await?;

                tracing::info!("✅ Payment status updated via webhook resend: {} -> {}",
                    payment.status.to_string(), new_status.to_string());
            }

            Ok(Json(json!({
                "success": true,
                "message": "Webhook resent and status updated",
                "payment_id": payment_id,
                "transaction_id": transaction_id,
                "new_status": new_status.to_string(),
                "timestamp": Utc::now().to_rfc3339()
            })))
        },
        Err(e) => {
            tracing::error!("❌ Failed to check Midtrans status for payment {}: {}", payment_id, e);
            Ok(Json(json!({
                "success": false,
                "message": "Failed to check payment status",
                "error": e.to_string(),
                "payment_id": payment_id
            })))
        }
    }
}