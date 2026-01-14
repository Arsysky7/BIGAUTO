use crate::config::AppState;
use crate::domain::models::{TransactionsResponse, TransactionLog, TransactionsQuery};
use crate::error::AppError;
use crate::middleware::AuthSeller;
use axum::{extract::{Query, State}, Json};
use sqlx::Row;

// Ambil history transaksi seller dengan pagination dan filter
#[utoipa::path(
    get,
    path = "/api/seller/transactions",
    responses(
        (status = 200, description = "Berhasil mengambil history transaksi", body = TransactionsResponse),
        (status = 401, description = "Unauthorized - JWT token invalid or missing"),
        (status = 403, description = "Forbidden - User is not a seller")
    ),
    params(
        ("transaction_type" = Option<String>, Query, description = "Filter by transaction type (rental_payment, sale_payment, seller_withdrawal, commission_deduction)"),
        ("limit" = Option<i64>, Query, description = "Limit number of results (default: 50, max: 100)"),
        ("offset" = Option<i64>, Query, description = "Offset for pagination (default: 0)")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Seller Transactions"
)]
pub async fn get_transactions(
    State(state): State<AppState>,
    auth: AuthSeller,
    Query(params): Query<TransactionsQuery>,
) -> Result<Json<TransactionsResponse>, AppError> {
    let seller_id = auth.user_id;
    let limit = params.limit.unwrap_or(50).min(100); 
    let offset = params.offset.unwrap_or(0);

    tracing::debug!(
        "Fetching transactions for seller_id: {} - type: {:?}, limit: {}, offset: {}",
        seller_id,
        params.transaction_type,
        limit,
        offset
    );

    // Build query dinamis berdasarkan filter
    let mut base_query = String::from(
        "SELECT 
        id, 
        transaction_type, 
        amount, 
        commission_amount, 
        net_amount, 
        status, 
        notes, 
        created_at 
        FROM transaction_logs 
        WHERE user_id = $1
        "
    );

    let mut count_query = String::from("SELECT COUNT(*) FROM transaction_logs WHERE user_id = $1");

    // Add transaction_type filter jika ada
    let use_type_filter = params.transaction_type.is_some();
    if use_type_filter {
        base_query.push_str(" AND transaction_type = $2");
        count_query.push_str(" AND transaction_type = $2");
    }

    // Order by created_at DESC (terbaru dulu)
    base_query.push_str(" ORDER BY created_at DESC");

    // Add pagination
    base_query.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));

    // Execute query untuk transactions
    let mut query_builder = sqlx::query(&base_query)
        .bind(seller_id);

    if let Some(ref tx_type) = params.transaction_type {
        query_builder = query_builder.bind(tx_type);
    }

    let rows = query_builder
        .fetch_all(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch transactions for seller_id {}: {}", seller_id, e);
            AppError::DatabaseError(format!("Gagal mengambil transaksi: {}", e))
        })?;

    // Map rows ke TransactionLog 
    let transactions: Vec<TransactionLog> = rows
        .into_iter()
        .map(|row| {
            // Parse columns manually 
            let transaction_type_str: String = row.get("transaction_type");
            let amount_f64: f64 = row.get("amount");
            let commission_amount: Option<f64> = row.try_get("commission_amount").ok();
            let net_amount: Option<f64> = row.try_get("net_amount").ok();
            let status_str: String = row.get("status");
            let notes: Option<String> = row.try_get("notes").ok();
            let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
            let id: i32 = row.get("id");

            // Convert strings ke type-safe enums
            use crate::domain::models::{TransactionType, TransactionStatus};

            TransactionLog {
                id,
                transaction_type: TransactionType::from(transaction_type_str.as_str()),
                amount: amount_f64,
                commission_amount,
                net_amount,
                status: TransactionStatus::from(status_str.as_str()),
                notes,
                created_at,
            }
        })
        .collect();

    // Get total count untuk pagination
    let mut count_builder = sqlx::query_scalar::<_, i64>(&count_query)
        .bind(seller_id);

    if let Some(ref tx_type) = params.transaction_type {
        count_builder = count_builder.bind(tx_type);
    }

    let total = count_builder
        .fetch_one(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to count transactions for seller_id {}: {}", seller_id, e);
            AppError::DatabaseError(format!("Gagal menghitung transaksi: {}", e))
        })?;

    tracing::info!(
        "Retrieved {} transactions for seller_id {} (total: {})",
        transactions.len(),
        seller_id,
        total
    );

    Ok(Json(TransactionsResponse {
        transactions,
        total,
        limit,
        offset,
    }))
}