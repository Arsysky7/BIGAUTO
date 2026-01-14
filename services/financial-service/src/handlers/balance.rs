use crate::config::AppState;
use crate::domain::models::BalanceResponse;
use crate::error::AppError;
use crate::middleware::AuthSeller;
use axum::{extract::State, Json};
use sqlx::query_scalar;

// Cek apakah seller punya balance record
async fn seller_exists(db: &sqlx::PgPool, seller_id: i32) -> Result<bool, AppError> {
    let exists = query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM seller_balance WHERE seller_id = $1)"
    )
    .bind(seller_id)
    .fetch_one(db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to check seller existence: {}", e);
        AppError::DatabaseError(e.to_string())
    })?;

    Ok(exists)
}

// Ambil saldo seller dari database
#[utoipa::path(
    get,
    path = "/api/seller/balance",
    responses(
        (status = 200, description = "Berhasil mengambil saldo seller", body = BalanceResponse),
        (status = 401, description = "Unauthorized - JWT token invalid or missing"),
        (status = 403, description = "Forbidden - User is not a seller"),
        (status = 404, description = "Balance record not found for this seller")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Seller Balance"
)]
pub async fn get_balance(
    State(state): State<AppState>,
    auth: AuthSeller,
) -> Result<Json<BalanceResponse>, AppError> {
    let seller_id = auth.user_id;

    tracing::debug!("Fetching balance for seller_id: {}", seller_id);

    // Cek apakah seller punya balance record
    let exists = seller_exists(&state.db, seller_id).await?;
    if !exists {
        tracing::warn!("Balance not found for seller_id: {}", seller_id);
        return Err(AppError::NotFoundError("Balance record tidak ditemukan".to_string()));
    }

    // Query balance dari database 
    let row = sqlx::query!(
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
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch balance for seller_id {}: {}", seller_id, e);
        AppError::DatabaseError(format!("Gagal mengambil saldo: {}", e))
    })?;

    tracing::info!(
        "Balance fetched for seller_id {} - available: {}, pending: {}, total: {}",
        seller_id,
        row.available_balance,
        row.pending_balance,
        row.total_earned
    );

    Ok(Json(BalanceResponse {
        seller_id: row.seller_id,
        available_balance: row.available_balance,
        pending_balance: row.pending_balance,
        total_earned: row.total_earned,
        updated_at: row.updated_at,
    }))
}