// Notification Handlers - Big Auto Notification Service

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use crate::{
    config::AppState,
    domain::notification::{NotificationResponse, MarkReadResponse, ReadAllResponse, UnreadCountResponse},
    error::{AppError, AppResult},
    middleware::auth::AuthUser,
};

/// Pagination query parameters
#[derive(Debug, Deserialize, ToSchema)]
pub struct NotificationQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

/// Get notifications untuk authenticated user
#[utoipa::path(
    get,
    path = "/api/notifications",
    tag = "Notifications",
    security(("bearer_auth" = [])),
    params(
        ("page" = Option<u32>, Query, description = "Page number (default: 1)"),
        ("limit" = Option<u32>, Query, description = "Items per page (default: 20)")
    ),
    responses(
        (status = 200, description = "Notifications retrieved successfully", body = NotificationListResponse),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_notifications(
    State(state): State<AppState>,
    AuthUser { user_id, .. }: AuthUser,
    Query(query): Query<NotificationQuery>,
) -> AppResult<Json<NotificationListResponse>> {
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).min(100).max(1);
    let offset = (page - 1) * limit;

    // Query notifications dari database
    let notifications = sqlx::query!(
        r#"
        SELECT id, type, title, message, related_id, related_type, is_read, read_at, created_at
        FROM notifications
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#,
        user_id,
        limit as i64,
        offset as i64
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch notifications for user {}: {}", user_id, e);
        AppError::internal("Gagal mengambil notifikasi")
    })?;

    // Get total count
    let total_count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM notifications WHERE user_id = $1",
        user_id
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to count notifications for user {}: {}", user_id, e);
        AppError::internal("Gagal menghitung notifikasi")
    })?
    .unwrap_or(0);

    let notification_responses: Vec<NotificationResponse> = notifications
        .into_iter()
        .map(|n| NotificationResponse {
            id: n.id,
            notification_type: n.r#type,
            title: n.title,
            message: n.message,
            related_id: n.related_id,
            related_type: n.related_type,
            is_read: n.is_read.unwrap_or(false),
            read_at: n.read_at,
            created_at: n.created_at.unwrap_or_else(|| chrono::Utc::now()),
        })
        .collect();

    Ok(Json(NotificationListResponse {
        data: notification_responses,
        total: total_count as u32,
        page,
        limit,
    }))
}

/// Response untuk notification list
#[derive(Debug, Serialize, ToSchema)]
pub struct NotificationListResponse {
    pub data: Vec<NotificationResponse>,
    pub total: u32,
    pub page: u32,
    pub limit: u32,
}

/// Mark single notification as read
#[utoipa::path(
    put,
    path = "/api/notifications/{id}/read",
    tag = "Notifications",
    security(("bearer_auth" = [])),
    params(
        ("id" = i32, Path, description = "Notification ID")
    ),
    responses(
        (status = 200, description = "Notification marked as read", body = MarkReadResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Notification not found")
    )
)]
pub async fn mark_as_read(
    State(state): State<AppState>,
    AuthUser { user_id, .. }: AuthUser,
    Path(id): Path<i32>,
) -> AppResult<Json<MarkReadResponse>> {
    // Cek apakah notification milik user
    let notification_exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM notifications WHERE id = $1 AND user_id = $2)",
        id,
        user_id
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to check notification existence: {}", e);
        AppError::internal("Gagal memeriksa notifikasi")
    })?
    .unwrap_or(false);

    if !notification_exists {
        return Err(AppError::not_found("Notification tidak ditemukan"));
    }

    // Update notification sebagai read
    sqlx::query!(
        r#"
        UPDATE notifications
        SET is_read = true, read_at = NOW()
        WHERE id = $1 AND user_id = $2
        "#,
        id,
        user_id
    )
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to mark notification as read: {}", e);
        AppError::internal("Gagal menandai notifikasi sebagai dibaca")
    })?;

    Ok(Json(MarkReadResponse {
        message: "Notifikasi ditandai sebagai dibaca".to_string(),
    }))
}

/// Mark all notifications as read
#[utoipa::path(
    put,
    path = "/api/notifications/read-all",
    tag = "Notifications",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "All notifications marked as read", body = ReadAllResponse),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn mark_all_as_read(
    State(state): State<AppState>,
    AuthUser { user_id, .. }: AuthUser,
) -> AppResult<Json<ReadAllResponse>> {
    // Update semua unread notifications milik user
    let result = sqlx::query!(
        r#"
        UPDATE notifications
        SET is_read = true, read_at = NOW()
        WHERE user_id = $1 AND is_read = false
        "#,
        user_id
    )
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to mark all notifications as read: {}", e);
        AppError::internal("Gagal menandai semua notifikasi sebagai dibaca")
    })?;

    let affected_count = result.rows_affected();

    Ok(Json(ReadAllResponse {
        message: format!("{} notifikasi ditandai sebagai dibaca", affected_count),
        affected_count: affected_count as i64,
    }))
}

/// Get unread notification count
#[utoipa::path(
    get,
    path = "/api/notifications/unread-count",
    tag = "Notifications",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Unread count retrieved", body = UnreadCountResponse),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_unread_count(
    State(state): State<AppState>,
    AuthUser { user_id, .. }: AuthUser,
) -> AppResult<Json<UnreadCountResponse>> {
    let unread_count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM notifications WHERE user_id = $1 AND is_read = false",
        user_id
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to get unread count for user {}: {}", user_id, e);
        AppError::internal("Gagal menghitung notifikasi belum dibaca")
    })?
    .unwrap_or(0);

    Ok(Json(UnreadCountResponse {
        unread_count,
    }))
}