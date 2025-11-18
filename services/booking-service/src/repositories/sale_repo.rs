use sqlx::PgPool;

use crate::{
    domain::sale::{SaleOrder, CreateSaleOrderRequest, SaleStatus},
    error::AppError,
};

// Generate unique order ID untuk sale
async fn generate_sale_order_id(pool: &PgPool) -> Result<String, AppError> {
    loop {
        let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
        let random: i32 = rand::random::<i32>().abs() % 10000;
        let order_id = format!("SALE-{}-{:04}", timestamp, random);

        // Validasi unique
        let exists: (bool,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM sale_orders WHERE order_id = $1)"
        )
        .bind(&order_id)
        .fetch_one(pool)
        .await?;

        if !exists.0 {
            return Ok(order_id);
        }
        // If exists, loop again and generate new ID
    }
}

// Create sale order baru
pub async fn create_sale_order(
    pool: &PgPool,
    buyer_id: i32,
    seller_id: i32,
    asking_price: f64,
    payload: &CreateSaleOrderRequest,
) -> Result<SaleOrder, AppError> {
    let order_id = generate_sale_order_id(pool).await?;

    let final_price = payload.offer_price.unwrap_or(asking_price);

    let sale_order = sqlx::query_as(
        "INSERT INTO sale_orders (
            vehicle_id, buyer_id, seller_id, testdrive_booking_id,
            order_id, asking_price, offer_price, final_price,
            buyer_name, buyer_phone, buyer_email, buyer_address, buyer_notes,
            status
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14
        ) RETURNING *"
    )
    .bind(payload.vehicle_id)
    .bind(buyer_id)
    .bind(seller_id)
    .bind(payload.testdrive_booking_id)
    .bind(&order_id)
    .bind(asking_price)
    .bind(payload.offer_price)
    .bind(final_price)
    .bind(&payload.buyer_name)
    .bind(&payload.buyer_phone)
    .bind(&payload.buyer_email)
    .bind(&payload.buyer_address)
    .bind(&payload.buyer_notes)
    .bind(SaleStatus::PendingConfirmation.as_str())
    .fetch_one(pool)
    .await?;

    Ok(sale_order)
}

// Ambil sale order by ID
pub async fn find_sale_order_by_id(
    pool: &PgPool,
    id: i32,
) -> Result<Option<SaleOrder>, AppError> {
    let result = sqlx::query_as(
        "SELECT * FROM sale_orders WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(result)
}

// Ambil sale orders by buyer dengan pagination
pub async fn find_sale_orders_by_buyer(
    pool: &PgPool,
    buyer_id: i32,
    status: Option<String>,
    page: Option<i32>,
    limit: Option<i32>,
) -> Result<Vec<SaleOrder>, AppError> {
    let page = page.unwrap_or(1).max(1);
    let limit = limit.unwrap_or(10).min(100);
    let offset = (page - 1) * limit;

    let orders = if let Some(status_filter) = status {
        sqlx::query_as(
            "SELECT * FROM sale_orders
             WHERE buyer_id = $1 AND status = $2
             ORDER BY created_at DESC
             LIMIT $3 OFFSET $4"
        )
        .bind(buyer_id)
        .bind(status_filter)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as(
            "SELECT * FROM sale_orders
             WHERE buyer_id = $1
             ORDER BY created_at DESC
             LIMIT $2 OFFSET $3"
        )
        .bind(buyer_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?
    };

    Ok(orders)
}

// Ambil sale orders by seller dengan pagination
pub async fn find_sale_orders_by_seller(
    pool: &PgPool,
    seller_id: i32,
    status: Option<String>,
    page: Option<i32>,
    limit: Option<i32>,
) -> Result<Vec<SaleOrder>, AppError> {
    let page = page.unwrap_or(1).max(1);
    let limit = limit.unwrap_or(10).min(100);
    let offset = (page - 1) * limit;

    let orders = if let Some(status_filter) = status {
        sqlx::query_as(
            "SELECT * FROM sale_orders
             WHERE seller_id = $1 AND status = $2
             ORDER BY created_at DESC
             LIMIT $3 OFFSET $4"
        )
        .bind(seller_id)
        .bind(status_filter)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as(
            "SELECT * FROM sale_orders
             WHERE seller_id = $1
             ORDER BY created_at DESC
             LIMIT $2 OFFSET $3"
        )
        .bind(seller_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?
    };

    Ok(orders)
}

// Seller confirm sale order (accept atau counter offer)
pub async fn confirm_sale_order(
    pool: &PgPool,
    id: i32,
    counter_offer_price: Option<f64>,
    seller_notes: Option<String>,
) -> Result<SaleOrder, AppError> {
    let sale_order = if let Some(counter_price) = counter_offer_price {
        // Seller melakukan counter offer
        sqlx::query_as(
            "UPDATE sale_orders
             SET status = $4,
                 counter_offer_price = $1,
                 seller_notes = $2,
                 confirmed_at = NOW(),
                 updated_at = NOW()
             WHERE id = $3
             RETURNING *"
        )
        .bind(counter_price)
        .bind(seller_notes)
        .bind(id)
        .bind(SaleStatus::PendingConfirmation.as_str())
        .fetch_one(pool)
        .await?
    } else {
        // Seller langsung accept
        sqlx::query_as(
            "UPDATE sale_orders
             SET status = $3,
                 seller_notes = $1,
                 confirmed_at = NOW(),
                 updated_at = NOW()
             WHERE id = $2
             RETURNING *"
        )
        .bind(seller_notes)
        .bind(id)
        .bind(SaleStatus::PendingPayment.as_str())
        .fetch_one(pool)
        .await?
    };

    Ok(sale_order)
}

// Buyer accept counter offer
pub async fn accept_counter_offer(
    pool: &PgPool,
    id: i32,
) -> Result<SaleOrder, AppError> {
    let sale_order: SaleOrder = sqlx::query_as("SELECT * FROM sale_orders WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await?;

    if sale_order.counter_offer_price.is_none() {
        return Err(AppError::bad_request("Tidak ada counter offer"));
    }

    let updated = sqlx::query_as(
        "UPDATE sale_orders
         SET status = 'pending_payment',
             final_price = counter_offer_price,
             updated_at = NOW()
         WHERE id = $1
         RETURNING *"
    )
    .bind(id)
    .fetch_one(pool)
    .await?;

    Ok(updated)
}

// Reject sale order (seller)
pub async fn reject_sale_order(
    pool: &PgPool,
    id: i32,
    reject_reason: &str,
) -> Result<SaleOrder, AppError> {
    let sale_order = sqlx::query_as(
        "UPDATE sale_orders
         SET status = 'rejected',
             reject_reason = $1,
             rejected_at = NOW(),
             updated_at = NOW()
         WHERE id = $2
         RETURNING *"
    )
    .bind(reject_reason)
    .bind(id)
    .fetch_one(pool)
    .await?;

    Ok(sale_order)
}

// Cancel sale order (buyer)
pub async fn cancel_sale_order(
    pool: &PgPool,
    id: i32,
    cancel_reason: &str,
) -> Result<SaleOrder, AppError> {
    let sale_order = sqlx::query_as(
        "UPDATE sale_orders
         SET status = 'cancelled',
             cancel_reason = $1,
             cancelled_at = NOW(),
             updated_at = NOW()
         WHERE id = $2
         RETURNING *"
    )
    .bind(cancel_reason)
    .bind(id)
    .fetch_one(pool)
    .await?;

    Ok(sale_order)
}

// Upload KTP (buyer)
pub async fn upload_ktp(
    pool: &PgPool,
    id: i32,
    ktp_photo: &str,
) -> Result<SaleOrder, AppError> {
    let sale_order = sqlx::query_as(
        "UPDATE sale_orders
         SET buyer_ktp_photo = $1,
             updated_at = NOW()
         WHERE id = $2
         RETURNING *"
    )
    .bind(ktp_photo)
    .bind(id)
    .fetch_one(pool)
    .await?;

    Ok(sale_order)
}

// Update sale order status to paid
pub async fn mark_as_paid(
    pool: &PgPool,
    id: i32,
) -> Result<SaleOrder, AppError> {
    let sale_order = sqlx::query_as(
        "UPDATE sale_orders
         SET status = 'paid',
             paid_at = NOW(),
             updated_at = NOW()
         WHERE id = $1
         RETURNING *"
    )
    .bind(id)
    .fetch_one(pool)
    .await?;

    Ok(sale_order)
}

// Start document transfer (seller)
pub async fn start_document_transfer(
    pool: &PgPool,
    id: i32,
) -> Result<SaleOrder, AppError> {
    let sale_order = sqlx::query_as(
        "UPDATE sale_orders
         SET status = 'document_processing',
             document_transfer_started_at = NOW(),
             updated_at = NOW()
         WHERE id = $1
         RETURNING *"
    )
    .bind(id)
    .fetch_one(pool)
    .await?;

    Ok(sale_order)
}

// Update document transfer status (seller)
pub async fn update_document_status(
    pool: &PgPool,
    id: i32,
    bpkb_transferred: Option<bool>,
    stnk_transferred: Option<bool>,
    faktur_transferred: Option<bool>,
    pajak_transferred: Option<bool>,
) -> Result<SaleOrder, AppError> {
    let sale_order = sqlx::query_as(
        "UPDATE sale_orders
         SET bpkb_transferred = COALESCE($1, bpkb_transferred),
             stnk_transferred = COALESCE($2, stnk_transferred),
             faktur_transferred = COALESCE($3, faktur_transferred),
             pajak_transferred = COALESCE($4, pajak_transferred),
             updated_at = NOW()
         WHERE id = $5
         RETURNING *"
    )
    .bind(bpkb_transferred)
    .bind(stnk_transferred)
    .bind(faktur_transferred)
    .bind(pajak_transferred)
    .bind(id)
    .fetch_one(pool)
    .await?;

    Ok(sale_order)
}

// Complete sale order (seller)
pub async fn complete_sale_order(
    pool: &PgPool,
    id: i32,
    seller_notes: Option<String>,
) -> Result<SaleOrder, AppError> {
    let sale_order = sqlx::query_as(
        "UPDATE sale_orders
         SET status = 'completed',
             seller_notes = COALESCE($1, seller_notes),
             completed_at = NOW(),
             updated_at = NOW()
         WHERE id = $2
         RETURNING *"
    )
    .bind(seller_notes)
    .bind(id)
    .fetch_one(pool)
    .await?;

    Ok(sale_order)
}
