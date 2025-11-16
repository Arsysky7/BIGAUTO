use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

use crate::{
    config::AppState,
    domain::session as session_domain,
    error::AppResult,
    middleware::auth::extract_authenticated_user,
};

// ===== RESPONSE DTOs =====

/// Response untuk satu session
#[derive(Debug, Serialize, ToSchema)]
pub struct SessionResponse {
    /// Session ID
    #[schema(example = 1)]
    pub id: i32,
    /// User ID pemilik session
    #[schema(example = 1)]
    pub user_id: i32,
    /// Nama device (dari user agent)
    #[schema(example = "Chrome on Windows")]
    pub device_name: Option<String>,
    /// IP address saat login
    #[schema(example = "192.168.1.1")]
    pub ip_address: Option<String>,
    /// Waktu aktivitas terakhir
    pub last_activity: Option<DateTime<Utc>>,
    /// Waktu expired session
    pub expires_at: DateTime<Utc>,
    /// Apakah ini session yang sedang aktif
    #[schema(example = true)]
    pub is_current: bool,
}

/// Response dengan message sukses
#[derive(Debug, Serialize, ToSchema)]
pub struct MessageResponse {
    /// Pesan sukses
    #[schema(example = "Operasi berhasil")]
    pub message: String,
}

// ===== HANDLER FUNCTIONS =====

/// Get all active sessions for the authenticated user
#[utoipa::path(
    get,
    path = "/api/auth/sessions",
    responses(
        (status = 200, description = "Successfully retrieved all active sessions", body = Vec<SessionResponse>),
    ),
    tag = "Sessions",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_sessions_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl IntoResponse> {
    // Extract user dari JWT token
    let auth_user = extract_authenticated_user(&headers, &state.config.jwt_secret)
        .map_err(|(_status, msg)| crate::error::AppError::authentication(&msg))?;

    // Call domain layer untuk get all active sessions
    let sessions = session_domain::get_active_sessions(&state, auth_user.user_id).await?;

    // Convert model ke response DTO
    let response: Vec<SessionResponse> = sessions
        .into_iter()
        .map(|s| SessionResponse {
            id: s.id,
            user_id: s.user_id,
            device_name: s.device_name,
            ip_address: s.ip_address,
            last_activity: s.last_activity,
            expires_at: s.expires_at,
            is_current: false, // TODO: bisa dibandingkan dengan current session token dari JWT
        })
        .collect();

    Ok(Json(response))
}

/// Invalidate a specific session by ID
#[utoipa::path(
    delete,
    path = "/api/auth/sessions/{id}",
    params(
        ("id" = i32, Path, description = "Session ID")
    ),
    responses(
        (status = 200, description = "Session successfully invalidated", body = MessageResponse),
    ),
    tag = "Sessions",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn invalidate_session_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(session_id): Path<i32>,
) -> AppResult<impl IntoResponse> {
    // Extract user dari JWT token
    let auth_user = extract_authenticated_user(&headers, &state.config.jwt_secret)
        .map_err(|(_status, msg)| crate::error::AppError::authentication(&msg))?;

    // Call domain layer untuk invalidate session (includes authorization check)
    session_domain::invalidate_session(&state, auth_user.user_id, session_id).await?;

    let response = MessageResponse {
        message: "Session berhasil dihapus. Device telah logout.".to_string(),
    };

    Ok(Json(response))
}

/// Invalidate all sessions for the authenticated user
#[utoipa::path(
    post,
    path = "/api/auth/sessions/invalidate-all",
    responses(
        (status = 200, description = "All sessions successfully invalidated", body = MessageResponse),
    ),
    tag = "Sessions",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn invalidate_all_sessions_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl IntoResponse> {
    // Extract user dari JWT token
    let auth_user = extract_authenticated_user(&headers, &state.config.jwt_secret)
        .map_err(|(_status, msg)| crate::error::AppError::authentication(&msg))?;

    // Call domain layer untuk invalidate all sessions
    session_domain::invalidate_all_sessions(&state, auth_user.user_id).await?;

    let response = MessageResponse {
        message: "Berhasil logout dari semua device.".to_string(),
    };

    Ok(Json(response))
}
