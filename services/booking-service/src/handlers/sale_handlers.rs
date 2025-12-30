// API Handlers untuk Sale Orders - Flow pembelian mobil dari customer ke seller
// Minimal working implementation for MVP
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};

use crate::{
    domain::sale::{
        CreateSaleOrderRequest, SaleOrderResponse, UpdateDocumentStatusRequest,
        SaleOrderQueryParams, UploadKtpRequest, AcceptSaleOrderRequest,
        AcceptCounterOfferRequest, CounterOfferRequest, CancelRequest,
        RejectSaleOrderRequest, StartDocumentTransferRequest, SaleStatus
    },
    middleware::auth::{AuthUser, AuthSeller, AuthCustomer},
    repositories::sale_repo,
    error::AppError,
    AppState,
};


// Create sale order baru (customer)
#[utoipa::path(
    post,
    path = "/api/sales/orders",
    tag = "sale-orders",
    summary = "Buat order pembelian mobil",
    description = "Customer membuat order pembelian mobil baru",
    security(
        ("bearer_auth" = [])
    ),
    request_body = CreateSaleOrderRequest,
    responses(
        (status = 201, description = "Order berhasil dibuat", body = SaleOrderResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 409, description = "Conflict - Vehicle not available or already ordered")
    )
)]
pub async fn create_sale_order(
    State(state): State<AppState>,
    auth: AuthCustomer,
    Json(request): Json<CreateSaleOrderRequest>,
) -> Result<(StatusCode, Json<SaleOrderResponse>), AppError> {
    // Validasi vehicle dan dapatkan seller_id + asking_price dari vehicle-service API
    let url = format!("{}/vehicles/{}/sale-info",
        state.config.vehicle_service_url,
        request.vehicle_id
    );

    #[derive(serde::Deserialize)]
    struct VehicleSaleInfo {
        seller_id: i32,
        asking_price: f64,
        is_available: bool,
    }

    let response = state.http_client
        .get(&url)
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| AppError::database_error(format!("Gagal menghubungi vehicle-service: {}", e)))?;

    if !response.status().is_success() {
        return Err(AppError::NotFound("Vehicle tidak ditemukan atau tidak tersedia untuk dijual".to_string()));
    }

    let vehicle_info: VehicleSaleInfo = response
        .json()
        .await
        .map_err(|e| AppError::database_error(format!("Gagal parse response: {}", e)))?;

    if !vehicle_info.is_available {
        return Err(AppError::Conflict("Vehicle tidak tersedia untuk dijual".to_string()));
    }

    // Buat sale order baru dengan data real dari vehicle-service
    let sale_order = sale_repo::create_sale_order(
        &state.db,
        auth.user_id,
        vehicle_info.seller_id,
        vehicle_info.asking_price,
        &request,
    )
    .await?;

    let response = SaleOrderResponse::from(sale_order);

    Ok((StatusCode::CREATED, Json(response)))
}

// Mendapatkan detail order pembelian (customer/seller yang terkait)
#[utoipa::path(
    get,
    path = "/api/sales/orders/{id}",
    tag = "sale-orders",
    summary = "Detail order pembelian",
    description = "Mendapatkan detail lengkap order pembelian mobil",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = i64, Path, description = "ID order pembelian")
    ),
    responses(
        (status = 200, description = "Detail order pembelian", body = SaleOrderResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Pesanan tidak ditemukan")
    )
)]
pub async fn get_sale_order(
    State(state): State<AppState>,
    Path(order_id): Path<i64>,
    auth: AuthUser,
) -> Result<Json<SaleOrderResponse>, AppError> {
    let sale_order = sale_repo::find_sale_order_by_id(&state.db, order_id as i32)
        .await?
        .ok_or(AppError::NotFound("Pesanan tidak ditemukan".to_string()))?;

    // Validasi akses: hanya customer atau seller terkait yang bisa melihat
    if auth.user_id != sale_order.buyer_id && auth.user_id != sale_order.seller_id {
        return Err(AppError::Forbidden("Akses ditolak".to_string()));
    }

    let response = SaleOrderResponse::from(sale_order);
    Ok(Json(response))
}

// Mendapatkan list order customer (customer perspective)
#[utoipa::path(
    get,
    path = "/api/sales/orders/my",
    tag = "sale-orders",
    summary = "List order pembelian saya",
    description = "Customer melihat semua order pembelian yang pernah dibuat",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "List order customer"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_customer_sale_orders(
    State(state): State<AppState>,
    auth: AuthCustomer,
    Query(params): Query<SaleOrderQueryParams>,
) -> Result<Json<Vec<SaleOrderResponse>>, AppError> {
    let orders = sale_repo::find_sale_orders_by_buyer(
        &state.db,
        auth.user_id,
        params.status,
        params.page,
        params.limit,
    ).await?;

    let response: Vec<SaleOrderResponse> = orders
        .into_iter()
        .map(SaleOrderResponse::from)
        .collect();

    Ok(Json(response))
}

// Mendapatkan list order seller (seller perspective)
#[utoipa::path(
    get,
    path = "/api/sales/orders/seller",
    tag = "sale-orders",
    summary = "List order penjualan saya",
    description = "Seller melihat semua order pembelian yang masuk ke mobil mereka",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "List order seller"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_seller_sale_orders(
    State(state): State<AppState>,
    auth: AuthSeller,
    Query(params): Query<SaleOrderQueryParams>,
) -> Result<Json<Vec<SaleOrderResponse>>, AppError> {
    let orders = sale_repo::find_sale_orders_by_seller(
        &state.db,
        auth.user_id,
        params.status,
        params.page,
        params.limit,
    ).await?;

    let response: Vec<SaleOrderResponse> = orders
        .into_iter()
        .map(SaleOrderResponse::from)
        .collect();

    Ok(Json(response))
}

// Confirm order (customer)
#[utoipa::path(
    put,
    path = "/api/sales/orders/{id}/confirm",
    tag = "sale-orders",
    summary = "Konfirmasi order pembelian",
    description = "Customer mengkonfirmasi order pembelian yang sudah dibuat",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = i64, Path, description = "ID order pembelian")
    ),
    responses(
        (status = 200, description = "Order berhasil dikonfirmasi", body = SaleOrderResponse),
        (status = 400, description = "Status tidak valid"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Pesanan tidak ditemukan")
    )
)]
pub async fn confirm_sale_order(
    State(state): State<AppState>,
    Path(order_id): Path<i64>,
    auth: AuthCustomer,
    Json(payload): Json<AcceptSaleOrderRequest>,
) -> Result<Json<SaleOrderResponse>, AppError> {
    // Cek order ada dan customer memiliki akses
    let sale_order = sale_repo::find_sale_order_by_id(&state.db, order_id as i32)
        .await?
        .ok_or(AppError::NotFound("Pesanan tidak ditemukan".to_string()))?;

    // Validasi akses: hanya buyer yang bisa konfirmasi
    if auth.user_id != sale_order.buyer_id {
        return Err(AppError::Forbidden("Akses ditolak - hanya pembeli yang bisa konfirmasi order".to_string()));
    }

    // Validasi status order
    if sale_order.status != "pending_confirmation" {
        return Err(AppError::BadRequest("Order tidak bisa dikonfirmasi - status tidak valid".to_string()));
    }

    // Validasi payload
    if payload.accept {
        // Customer menerima harga
        let updated_order = sale_repo::confirm_sale_order(
            &state.db,
            order_id as i32,
            None, // counter_price - tidak ada
            payload.notes.clone(),
        ).await?;

        Ok(Json(SaleOrderResponse::from(updated_order)))
    } else {
        // Customer menolak harga (implementasi di endpoint reject)
        Err(AppError::BadRequest("Silakan gunakan endpoint /cancel untuk menolak pesanan".to_string()))
    }
}

// Seller counter offer
#[utoipa::path(
    put,
    path = "/api/sales/orders/{id}/counter",
    tag = "sale-orders",
    summary = "Counter offer dari seller",
    description = "Seller memberikan counter offer harga",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = i64, Path, description = "ID order pembelian")
    ),
    request_body = CounterOfferRequest,
    responses(
        (status = 200, description = "Counter offer berhasil dibuat", body = SaleOrderResponse),
        (status = 400, description = "Status tidak valid"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Pesanan tidak ditemukan")
    )
)]
pub async fn seller_counter_offer(
    State(state): State<AppState>,
    Path(order_id): Path<i64>,
    auth: AuthSeller,
    Json(payload): Json<CounterOfferRequest>,
) -> Result<Json<SaleOrderResponse>, AppError> {
    // Cek order ada dan seller memiliki akses
    let sale_order = sale_repo::find_sale_order_by_id(&state.db, order_id as i32)
        .await?
        .ok_or(AppError::NotFound("Pesanan tidak ditemukan".to_string()))?;

    if auth.user_id != sale_order.seller_id {
        return Err(AppError::Forbidden("Akses ditolak".to_string()));
    }

    // Validasi status order menggunakan SaleStatus enum
    if SaleStatus::from_str(&sale_order.status) != Some(SaleStatus::PendingConfirmation) {
        return Err(AppError::BadRequest("Hanya bisa memberikan counter offer untuk order yang menunggu konfirmasi".to_string()));
    }

    // Validasi counter offer price
    let counter_price = payload.counter_price;
    if counter_price <= 0.0 {
        return Err(AppError::BadRequest("Harga counter offer harus positif".to_string()));
    }

    // Lakukan counter offer
    let updated_order = sale_repo::confirm_sale_order(
        &state.db,
        order_id as i32,
        Some(counter_price),
        payload.reason.clone(),
    ).await?;

    Ok(Json(SaleOrderResponse::from(updated_order)))
}

// Reject sale order (seller)
#[utoipa::path(
    put,
    path = "/api/sales/orders/{id}/reject",
    tag = "sale-orders",
    summary = "Tolak order pembelian",
    description = "Seller menolak order pembelian",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = i64, Path, description = "ID order pembelian")
    ),
    request_body = RejectSaleOrderRequest,
    responses(
        (status = 200, description = "Order berhasil ditolak", body = SaleOrderResponse),
        (status = 400, description = "Status tidak valid"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Pesanan tidak ditemukan")
    )
)]
pub async fn reject_sale_order(
    State(state): State<AppState>,
    Path(order_id): Path<i64>,
    auth: AuthSeller,
    Json(payload): Json<RejectSaleOrderRequest>,
) -> Result<Json<SaleOrderResponse>, AppError> {
    // Cek order ada dan seller memiliki akses
    let sale_order = sale_repo::find_sale_order_by_id(&state.db, order_id as i32)
        .await?
        .ok_or(AppError::NotFound("Pesanan tidak ditemukan".to_string()))?;

    if auth.user_id != sale_order.seller_id {
        return Err(AppError::Forbidden("Akses ditolak".to_string()));
    }

    // Validasi status order menggunakan SaleStatus enum
    if SaleStatus::from_str(&sale_order.status) != Some(SaleStatus::PendingConfirmation) {
        return Err(AppError::BadRequest("Hanya bisa menolak order yang menunggu konfirmasi".to_string()));
    }

    // Validasi alasan penolakan
    let reject_reason = if payload.reject_reason.trim().is_empty() {
        "Ditolak oleh seller".to_string()
    } else {
        payload.reject_reason.clone()
    };

    // Tolak order
    let updated_order = sale_repo::reject_sale_order(
        &state.db,
        order_id as i32,
        &reject_reason,
    ).await?;

    Ok(Json(SaleOrderResponse::from(updated_order)))
}

// Accept counter offer (customer)
#[utoipa::path(
    put,
    path = "/api/sales/orders/{id}/accept-counter",
    tag = "sale-orders",
    summary = "Terima counter offer",
    description = "Customer menerima counter offer dari seller",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = i64, Path, description = "ID order pembelian")
    ),
    responses(
        (status = 200, description = "Counter offer diterima", body = SaleOrderResponse),
        (status = 400, description = "Status tidak valid"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Pesanan tidak ditemukan")
    )
)]
pub async fn accept_counter_offer(
    State(state): State<AppState>,
    Path(order_id): Path<i64>,
    auth: AuthCustomer,
    Json(payload): Json<AcceptCounterOfferRequest>,
) -> Result<Json<SaleOrderResponse>, AppError> {
    // Validasi bahwa customer menerima counter offer
    if !payload.accept {
        return Err(AppError::BadRequest("Payload harus menunjukkan penerimaan".to_string()));
    }

    // Cek order ada dan customer memiliki akses
    let sale_order = sale_repo::find_sale_order_by_id(&state.db, order_id as i32)
        .await?
        .ok_or(AppError::NotFound("Pesanan tidak ditemukan".to_string()))?;

    if auth.user_id != sale_order.buyer_id {
        return Err(AppError::Forbidden("Akses ditolak".to_string()));
    }

    // Terima counter offer
    let updated_order = sale_repo::accept_counter_offer(
        &state.db,
        order_id as i32,
    ).await?;

    Ok(Json(SaleOrderResponse::from(updated_order)))
}

// Cancel order (customer/seller)
#[utoipa::path(
    put,
    path = "/api/sales/orders/{id}/cancel",
    tag = "sale-orders",
    summary = "Batalkan order pembelian",
    description = "Customer atau seller membatalkan order pembelian",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = i64, Path, description = "ID order pembelian")
    ),
    responses(
        (status = 200, description = "Order berhasil dibatalkan", body = SaleOrderResponse),
        (status = 400, description = "Status tidak valid"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Pesanan tidak ditemukan")
    )
)]
pub async fn cancel_sale_order(
    State(state): State<AppState>,
    Path(order_id): Path<i64>,
    auth: AuthUser,
    Json(payload): Json<CancelRequest>,
) -> Result<Json<SaleOrderResponse>, AppError> {
    // Cek order ada dan user memiliki akses
    let sale_order = sale_repo::find_sale_order_by_id(&state.db, order_id as i32)
        .await?
        .ok_or(AppError::NotFound("Pesanan tidak ditemukan".to_string()))?;

    // Validasi akses: hanya buyer atau seller yang bisa cancel
    if auth.user_id != sale_order.buyer_id && auth.user_id != sale_order.seller_id {
        return Err(AppError::Forbidden("Akses ditolak".to_string()));
    }

    // Validasi alasan pembatalan
    let cancel_reason = if payload.reason.trim().is_empty() {
        "Dibatalkan oleh user".to_string()
    } else {
        payload.reason.clone()
    };

    // Batalkan order
    let updated_order = sale_repo::cancel_sale_order(
        &state.db,
        order_id as i32,
        &cancel_reason,
    ).await?;

    Ok(Json(SaleOrderResponse::from(updated_order)))
}

// Upload KTP (customer)
#[utoipa::path(
    put,
    path = "/api/sales/orders/{id}/upload-ktp",
    tag = "sale-orders",
    summary = "Upload KTP pembeli",
    description = "Customer mengupload foto KTP untuk verifikasi",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = i64, Path, description = "ID order pembelian")
    ),
    request_body = UploadKtpRequest,
    responses(
        (status = 200, description = "KTP berhasil diupload", body = SaleOrderResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Pesanan tidak ditemukan")
    )
)]
pub async fn upload_buyer_ktp(
    State(state): State<AppState>,
    Path(order_id): Path<i64>,
    auth: AuthCustomer,
    Json(payload): Json<UploadKtpRequest>,
) -> Result<Json<SaleOrderResponse>, AppError> {
    // Cek order ada dan customer memiliki akses
    let sale_order = sale_repo::find_sale_order_by_id(&state.db, order_id as i32)
        .await?
        .ok_or(AppError::NotFound("Pesanan tidak ditemukan".to_string()))?;

    if auth.user_id != sale_order.buyer_id {
        return Err(AppError::Forbidden("Akses ditolak".to_string()));
    }

    // Validasi URL KTP
    if payload.ktp_photo.is_empty() {
        return Err(AppError::BadRequest("URL foto KTP harus diisi".to_string()));
    }

    // Upload KTP
    let updated_order = sale_repo::upload_ktp(
        &state.db,
        order_id as i32,
        &payload.ktp_photo,
    ).await?;

    Ok(Json(SaleOrderResponse::from(updated_order)))
}

// Start document transfer (seller)
#[utoipa::path(
    put,
    path = "/api/sales/orders/{id}/start-documents",
    tag = "sale-orders",
    summary = "Mulai proses dokumen",
    description = "Seller memulai proses transfer dokumen kendaraan",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = i64, Path, description = "ID order pembelian")
    ),
    request_body = StartDocumentTransferRequest,
    responses(
        (status = 200, description = "Proses dokumen dimulai", body = SaleOrderResponse),
        (status = 400, description = "Status tidak valid"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Pesanan tidak ditemukan")
    )
)]
pub async fn start_document_transfer(
    State(state): State<AppState>,
    Path(order_id): Path<i64>,
    auth: AuthSeller,
    Json(payload): Json<StartDocumentTransferRequest>,
) -> Result<Json<SaleOrderResponse>, AppError> {
    // Cek order ada dan seller memiliki akses
    let sale_order = sale_repo::find_sale_order_by_id(&state.db, order_id as i32)
        .await?
        .ok_or(AppError::NotFound("Pesanan tidak ditemukan".to_string()))?;

    if auth.user_id != sale_order.seller_id {
        return Err(AppError::Forbidden("Akses ditolak".to_string()));
    }

    // Validasi status order menggunakan SaleStatus enum
    if SaleStatus::from_str(&sale_order.status) != Some(SaleStatus::Paid) {
        return Err(AppError::BadRequest("Hanya bisa memulai transfer dokumen untuk order yang sudah dibayar".to_string()));
    }

    // Log notes jika provided
    if let Some(notes) = &payload.notes {
        tracing::info!("Document transfer notes untuk order {}: {}", order_id, notes);
    }

    // Mulai proses transfer dokumen
    let updated_order = sale_repo::start_document_transfer(
        &state.db,
        order_id as i32,
    ).await?;

    Ok(Json(SaleOrderResponse::from(updated_order)))
}

// Update document status (seller)
#[utoipa::path(
    put,
    path = "/api/sales/orders/{id}/update-documents",
    tag = "sale-orders",
    summary = "Update status dokumen",
    description = "Seller mengupdate status transfer dokumen",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = i64, Path, description = "ID order pembelian")
    ),
    request_body = UpdateDocumentStatusRequest,
    responses(
        (status = 200, description = "Status dokumen diupdate", body = SaleOrderResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Pesanan tidak ditemukan")
    )
)]
pub async fn update_document_status(
    State(state): State<AppState>,
    Path(order_id): Path<i64>,
    auth: AuthSeller,
    Json(payload): Json<UpdateDocumentStatusRequest>,
) -> Result<Json<SaleOrderResponse>, AppError> {
    // Cek order ada dan seller memiliki akses
    let sale_order = sale_repo::find_sale_order_by_id(&state.db, order_id as i32)
        .await?
        .ok_or(AppError::NotFound("Pesanan tidak ditemukan".to_string()))?;

    if auth.user_id != sale_order.seller_id {
        return Err(AppError::Forbidden("Akses ditolak".to_string()));
    }

    // Validasi status order - hanya bisa update dokumen jika sedang dalam proses
    if sale_order.status != "document_processing" {
        return Err(AppError::BadRequest("Hanya bisa update dokumen untuk order yang sedang dalam proses dokumen".to_string()));
    }

    // Update status dokumen
    let updated_order = sale_repo::update_document_status(
        &state.db,
        order_id as i32,
        payload.bpkb_transferred,
        payload.stnk_transferred,
        payload.faktur_transferred,
        payload.pajak_transferred,
    ).await?;

    Ok(Json(SaleOrderResponse::from(updated_order)))
}

// Mark sale order as paid (payment callback from payment-service)
#[utoipa::path(
    put,
    path = "/api/sales/orders/{id}/mark-paid",
    tag = "sale-orders",
    summary = "Tandai order sebagai terbayar",
    description = "Payment callback dari payment-service untuk menandai order sebagai terbayar",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = i64, Path, description = "ID order pembelian")
    ),
    responses(
        (status = 200, description = "Order berhasil ditandai terbayar", body = SaleOrderResponse),
        (status = 400, description = "Status tidak valid"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Pesanan tidak ditemukan")
    )
)]
pub async fn mark_sale_order_as_paid(
    State(state): State<AppState>,
    Path(order_id): Path<i64>,
    _auth: AuthUser,  
) -> Result<Json<SaleOrderResponse>, AppError> {
    // Bisa diakses oleh customer/seller yang memiliki valid token
    // Validasi tambahan bisa ditambahkan untuk payment service signature

    // Cek order ada
    let sale_order = sale_repo::find_sale_order_by_id(&state.db, order_id as i32)
        .await?
        .ok_or(AppError::NotFound("Pesanan tidak ditemukan".to_string()))?;

    // Validasi status - hanya bisa mark as paid jika pending_payment
    if sale_order.status != "pending_payment" {
        return Err(AppError::BadRequest("Status order tidak valid untuk ditandai terbayar".to_string()));
    }

    // Mark as paid
    let updated_order = sale_repo::mark_as_paid(
        &state.db,
        order_id as i32,
    ).await?;

    Ok(Json(SaleOrderResponse::from(updated_order)))
}

// Confirm documents received (customer)
#[utoipa::path(
    put,
    path = "/api/sales/orders/{id}/confirm-documents",
    tag = "sale-orders",
    summary = "Konfirmasi dokumen diterima",
    description = "Customer mengkonfirmasi dokumen kendaraan sudah diterima",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = i64, Path, description = "ID order pembelian")
    ),
    responses(
        (status = 200, description = "Dokumen dikonfirmasi diterima", body = SaleOrderResponse),
        (status = 400, description = "Status tidak valid"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Pesanan tidak ditemukan")
    )
)]
pub async fn confirm_documents_received(
    State(state): State<AppState>,
    Path(order_id): Path<i64>,
    auth: AuthCustomer,
) -> Result<Json<SaleOrderResponse>, AppError> {
    // Cek order ada dan customer memiliki akses
    let sale_order = sale_repo::find_sale_order_by_id(&state.db, order_id as i32)
        .await?
        .ok_or(AppError::NotFound("Pesanan tidak ditemukan".to_string()))?;

    if auth.user_id != sale_order.buyer_id {
        return Err(AppError::Forbidden("Akses ditolak".to_string()));
    }

    // Validasi status order - hanya bisa konfirmasi dokumen jika sedang dalam proses
    if sale_order.status != "document_processing" {
        return Err(AppError::BadRequest("Hanya bisa konfirmasi dokumen untuk order yang sedang dalam proses dokumen".to_string()));
    }

    // Validasi bahwa semua dokumen sudah ditransfer
    if !sale_order.bpkb_transferred || !sale_order.stnk_transferred ||
       !sale_order.faktur_transferred || !sale_order.pajak_transferred {
        return Err(AppError::BadRequest("Semua dokumen harus ditransfer sebelum konfirmasi".to_string()));
    }

    // TODO: Tambahkan notifikasi ke seller dan update tracking
    // Fitur notifikasi dan tracking akan diimplement di kemudian hari
    if false { // Placeholder untuk future enhancement
        return Err(AppError::not_implemented("Fitur notifikasi dan tracking belum diimplementasi"));
    }

    // Selesaikan order
    let updated_order = sale_repo::complete_sale_order(
        &state.db,
        order_id as i32,
        Some("Dokumen dikonfirmasi diterima oleh pembeli".to_string()),
    ).await?;

    Ok(Json(SaleOrderResponse::from(updated_order)))
}