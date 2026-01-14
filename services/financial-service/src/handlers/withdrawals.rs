use crate::config::AppState;
use crate::domain::models::{CreateWithdrawalRequest, WithdrawalResponse, Withdrawal, WithdrawalsListResponse, WithdrawalsListQuery};
use crate::error::AppError;
use crate::middleware::AuthSeller;
use axum::{extract::{Path, Query, State}, Json};
use sqlx::Row;

// Buat request withdrawal baru untuk seller
#[utoipa::path(
    post,
    path = "/api/seller/withdrawals",
    responses(
        (status = 200, description = "Withdrawal request berhasil dibuat", body = WithdrawalResponse),
        (status = 400, description = "Request tidak valid (saldo tidak cukup / amount kurang dari minimum)"),
        (status = 401, description = "Unauthorized - JWT token invalid or missing"),
        (status = 403, description = "Forbidden - User is not a seller")
    ),
    request_body = CreateWithdrawalRequest,
    security(
        ("bearer_auth" = [])
    ),
    tag = "Seller Withdrawals"
)]
pub async fn create_withdrawal(
    State(state): State<AppState>,
    auth: AuthSeller,
    Json(payload): Json<CreateWithdrawalRequest>,
) -> Result<Json<WithdrawalResponse>, AppError> {
    let seller_id = auth.user_id;

    tracing::info!(
        "Seller {} requesting withdrawal: Rp {:.2}",
        seller_id,
        payload.amount
    );

    // Ambil MIN_WITHDRAWAL_AMOUNT dari env atau default 50.000
    let min_withdrawal: f64 = std::env::var("MIN_WITHDRAWAL_AMOUNT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(50000.0);

    // Validate amount minimum
    if payload.amount < min_withdrawal {
        return Err(AppError::validation(format!(
            "Minimum withdrawal adalah Rp {:.0}",
            min_withdrawal
        ).as_str()));
    }

    // Check saldo tersedia di seller_balance
    let balance_row = sqlx::query!(
        r#"
        SELECT
            seller_id as "seller_id!: i32",
            available_balance::FLOAT8 as "available_balance!: f64",
            pending_balance::FLOAT8 as "pending_balance!: f64",
            total_earned::FLOAT8 as "total_earned!: f64",
            updated_at as "updated_at!: chrono::DateTime<chrono::Utc>"
        FROM seller_balance
        WHERE seller_id = $1
        "#,
        seller_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch balance for seller_id {}: {}", seller_id, e);
        AppError::DatabaseError(format!("Gagal mengambil saldo: {}", e))
    })?;

    let available_balance = match balance_row {
        Some(row) => row.available_balance,
        None => {
            return Err(AppError::not_found("Saldo seller tidak ditemukan"));
        }
    };

    // Validate amount tidak melebihi saldo tersedia
    if payload.amount > available_balance {
        return Err(AppError::validation(format!(
            "Saldo tidak cukup. Tersedia: Rp {:.2}, Diminta: Rp {:.2}",
            available_balance,
            payload.amount
        ).as_str()));
    }

    // Start transaction untuk buat withdrawal record + update balance + create transaction log
    let mut tx = state.db.begin().await
        .map_err(|e| {
            tracing::error!("Failed to start transaction: {}", e);
            AppError::DatabaseError(format!("Gagal memulai transaksi: {}", e))
        })?;

    // 1. Insert ke withdrawals table
    let withdrawal_row = sqlx::query(
        r#"
        INSERT INTO withdrawals (seller_id, amount, bank_name, account_number, account_holder_name, status)
        VALUES ($1, $2, $3, $4, $5, 'pending')
        RETURNING
            id as "id!: i32",
            seller_id as "seller_id!: i32",
            amount::FLOAT8 as "amount!: f64",
            bank_name as "bank_name!: String",
            account_number as "account_number!: String",
            account_holder_name as "account_holder_name!: String",
            status as "status!: String",
            requested_at as "requested_at!: chrono::DateTime<chrono::Utc>"
        "#
    )
    .bind(seller_id)
    .bind(payload.amount)
    .bind(&payload.bank_name)
    .bind(&payload.account_number)
    .bind(&payload.account_holder_name)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create withdrawal for seller_id {}: {}", seller_id, e);
        AppError::DatabaseError(format!("Gagal membuat withdrawal: {}", e))
    })?;

    let withdrawal_id: i32 = withdrawal_row.get("id");

    // 2. Reduce available_balance di seller_balance
    sqlx::query(
        r#"
        UPDATE seller_balance
        SET
            available_balance = available_balance - $2,
            updated_at = NOW()
        WHERE seller_id = $1
        "#
    )
    .bind(seller_id)
    .bind(payload.amount)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!("Failed to update balance for seller_id {}: {}", seller_id, e);
        AppError::DatabaseError(format!("Gagal update saldo: {}", e))
    })?;

    // 3. Create transaction_log untuk seller_withdrawal
    sqlx::query(
        r#"
        INSERT INTO transaction_logs (transaction_type, user_id, withdrawal_id, amount, net_amount, status, notes)
        VALUES ('seller_withdrawal', $1, $2, $3, $4, 'pending', $5)
        "#
    )
    .bind(seller_id)
    .bind(withdrawal_id)
    .bind(payload.amount)
    .bind(-payload.amount) // Net amount negative untuk withdrawal
    .bind(format!("Withdrawal request #{}", withdrawal_id))
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create transaction_log: {}", e);
        AppError::DatabaseError(format!("Gagal membuat transaction log: {}", e))
    })?;

    // Commit transaction
    tx.commit().await
        .map_err(|e| {
            tracing::error!("Failed to commit transaction: {}", e);
            AppError::DatabaseError(format!("Gagal commit transaksi: {}", e))
        })?;

    tracing::info!(
        "Withdrawal request created successfully - id: {}, seller_id: {}, amount: Rp {:.2}",
        withdrawal_id,
        seller_id,
        payload.amount
    );

    Ok(Json(WithdrawalResponse {
        id: withdrawal_row.get("id"),
        seller_id: withdrawal_row.get("seller_id"),
        amount: withdrawal_row.get("amount"),
        bank_name: withdrawal_row.get("bank_name"),
        account_number: withdrawal_row.get("account_number"),
        account_holder_name: withdrawal_row.get("account_holder_name"),
        status: {
            let status_str: String = withdrawal_row.get("status");
            crate::domain::models::WithdrawalStatus::from(status_str.as_str())
        },
        requested_at: withdrawal_row.get("requested_at"),
    }))
}

// Get list withdrawal requests untuk seller
#[utoipa::path(
    get,
    path = "/api/seller/withdrawals",
    responses(
        (status = 200, description = "List withdrawal requests berhasil diambil", body = WithdrawalsListResponse),
        (status = 401, description = "Unauthorized - JWT token invalid or missing"),
        (status = 403, description = "Forbidden - User is not a seller")
    ),
    params(
        ("status" = Option<String>, Query, description = "Filter by status (pending, processing, completed, failed)"),
        ("limit" = Option<i64>, Query, description = "Jumlah item per halaman (default: 50, max: 100)"),
        ("offset" = Option<i64>, Query, description = "Offset untuk pagination (default: 0)")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Seller Withdrawals"
)]
pub async fn list_withdrawals(
    State(state): State<AppState>,
    auth: AuthSeller,
    Query(params): Query<WithdrawalsListQuery>,
) -> Result<Json<WithdrawalsListResponse>, AppError> {
    let seller_id = auth.user_id;

    tracing::info!(
        "Seller {} requesting withdrawals list with filters: {:?}",
        seller_id,
        params
    );

    // Parse pagination params dengan defaults
    let limit = params.limit.unwrap_or(50).min(100);
    let offset = params.offset.unwrap_or(0);

    // Build dynamic query dengan optional status filter
    let mut base_query = String::from(
        "SELECT
            id as \"id!: i32\",
            seller_id as \"seller_id!: i32\",
            amount::FLOAT8 as \"amount!: f64\",
            bank_name as \"bank_name!: String\",
            account_number as \"account_number!: String\",
            account_holder_name as \"account_holder_name!: String\",
            status as \"status!: String\",
            requested_at as \"requested_at!: chrono::DateTime<chrono::Utc>\",
            processed_at as \"processed_at: Option<chrono::DateTime<chrono::Utc>>\",
            completed_at as \"completed_at: Option<chrono::DateTime<chrono::Utc>>\"
        FROM withdrawals
        WHERE seller_id = $1"
    );

    // Add status filter if provided
    if params.status.is_some() {
        base_query.push_str(" AND status = $2");
    }

    // Count total query untuk pagination
    let count_query = if params.status.is_some() {
        "SELECT COUNT(*) FROM withdrawals WHERE seller_id = $1 AND status = $2"
    } else {
        "SELECT COUNT(*) FROM withdrawals WHERE seller_id = $1"
    };

    // Get total count
    let total: i64 = if let Some(ref status) = params.status {
        sqlx::query_scalar(count_query)
            .bind(seller_id)
            .bind(status)
            .fetch_one(&state.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to count withdrawals for seller_id {}: {}", seller_id, e);
                AppError::DatabaseError(format!("Gagal menghitung withdrawals: {}", e))
            })?
    } else {
        sqlx::query_scalar(count_query)
            .bind(seller_id)
            .fetch_one(&state.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to count withdrawals for seller_id {}: {}", seller_id, e);
                AppError::DatabaseError(format!("Gagal menghitung withdrawals: {}", e))
            })?
    };

    // Add ORDER BY and LIMIT/OFFSET
    base_query.push_str(" ORDER BY requested_at DESC");
    base_query.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));

    // Fetch withdrawals dengan dynamic query
    let withdrawals: Vec<Withdrawal> = if let Some(ref status) = params.status {
        let rows = sqlx::query(&base_query)
            .bind(seller_id)
            .bind(status)
            .fetch_all(&state.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to fetch withdrawals for seller_id {}: {}", seller_id, e);
                AppError::DatabaseError(format!("Gagal mengambil withdrawals: {}", e))
            })?;

        rows.into_iter()
            .map(|row| {
                let status_str: String = row.get("status");
                Withdrawal {
                    id: row.get("id"),
                    seller_id: row.get("seller_id"),
                    amount: row.get("amount"),
                    bank_name: row.get("bank_name"),
                    account_number: row.get("account_number"),
                    account_holder_name: row.get("account_holder_name"),
                    status: crate::domain::models::WithdrawalStatus::from(status_str.as_str()),
                    requested_at: row.get("requested_at"),
                    processed_at: row.try_get("processed_at").ok(),
                    completed_at: row.try_get("completed_at").ok(),
                }
            })
            .collect()
    } else {
        let rows = sqlx::query(&base_query)
            .bind(seller_id)
            .fetch_all(&state.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to fetch withdrawals for seller_id {}: {}", seller_id, e);
                AppError::DatabaseError(format!("Gagal mengambil withdrawals: {}", e))
            })?;

        rows.into_iter()
            .map(|row| {
                let status_str: String = row.get("status");
                Withdrawal {
                    id: row.get("id"),
                    seller_id: row.get("seller_id"),
                    amount: row.get("amount"),
                    bank_name: row.get("bank_name"),
                    account_number: row.get("account_number"),
                    account_holder_name: row.get("account_holder_name"),
                    status: crate::domain::models::WithdrawalStatus::from(status_str.as_str()),
                    requested_at: row.get("requested_at"),
                    processed_at: row.try_get("processed_at").ok(),
                    completed_at: row.try_get("completed_at").ok(),
                }
            })
            .collect()
    };

    tracing::info!(
        "Returning {} withdrawals for seller {} (total: {}, limit: {}, offset: {})",
        withdrawals.len(),
        seller_id,
        total,
        limit,
        offset
    );

    Ok(Json(WithdrawalsListResponse {
        withdrawals,
        total,
        limit,
        offset,
    }))
}

// Get detail withdrawal request by ID untuk seller
#[utoipa::path(
    get,
    path = "/api/seller/withdrawals/{id}",
    responses(
        (status = 200, description = "Withdrawal detail berhasil diambil", body = Withdrawal),
        (status = 401, description = "Unauthorized - JWT token invalid or missing"),
        (status = 403, description = "Forbidden - User is not a seller"),
        (status = 404, description = "Withdrawal tidak ditemukan atau bukan milik seller ini")
    ),
    params(
        ("id" = i32, Path, description = "Withdrawal ID")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Seller Withdrawals"
)]
pub async fn get_withdrawal_by_id(
    State(state): State<AppState>,
    auth: AuthSeller,
    Path(id): Path<i32>,
) -> Result<Json<Withdrawal>, AppError> {
    let seller_id = auth.user_id;

    tracing::info!(
        "Seller {} requesting withdrawal detail for id: {}",
        seller_id,
        id
    );

    // Fetch withdrawal dan verify ownership dalam satu query
    let withdrawal_row = sqlx::query(
        r#"
        SELECT
            id as "id!: i32",
            seller_id as "seller_id!: i32",
            amount::FLOAT8 as "amount!: f64",
            bank_name as "bank_name!: String",
            account_number as "account_number!: String",
            account_holder_name as "account_holder_name!: String",
            status as "status!: String",
            requested_at as "requested_at!: chrono::DateTime<chrono::Utc>",
            processed_at as "processed_at: Option<chrono::DateTime<chrono::Utc>>",
            completed_at as "completed_at: Option<chrono::DateTime<chrono::Utc>>"
        FROM withdrawals
        WHERE id = $1 AND seller_id = $2
        "#
    )
    .bind(id)
    .bind(seller_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch withdrawal {} for seller_id {}: {}", id, seller_id, e);
        AppError::DatabaseError(format!("Gagal mengambil withdrawal: {}", e))
    })?;

    let withdrawal = match withdrawal_row {
        Some(row) => {
            let status_str: String = row.get("status");
            Withdrawal {
                id: row.get("id"),
                seller_id: row.get("seller_id"),
                amount: row.get("amount"),
                bank_name: row.get("bank_name"),
                account_number: row.get("account_number"),
                account_holder_name: row.get("account_holder_name"),
                status: crate::domain::models::WithdrawalStatus::from(status_str.as_str()),
                requested_at: row.get("requested_at"),
                processed_at: row.try_get("processed_at").ok(),
                completed_at: row.try_get("completed_at").ok(),
            }
        },
        None => {
            return Err(AppError::not_found("Withdrawal tidak ditemukan atau bukan milik seller ini"));
        }
    };

    tracing::info!(
        "Returning withdrawal detail for id: {}, seller_id: {}, status: {}",
        withdrawal.id,
        withdrawal.seller_id,
        withdrawal.status
    );

    Ok(Json(withdrawal))
}