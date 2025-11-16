use crate::config::AppState;
use crate::error::AppError;
use crate::models::session::{UserSession};
use chrono::{DateTime, Utc};

// Struktur response data session
#[derive(Debug, serde::Serialize)]
pub struct SessionResponse {
    pub id: i32,
    pub user_id: i32,
    pub device_name: Option<String>,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    pub last_activity: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl From<UserSession> for SessionResponse {
    fn from(session: UserSession) -> Self {
        SessionResponse {
            id: session.id,
            user_id: session.user_id,
            device_name: session.device_name,
            user_agent: session.user_agent,
            ip_address: session.ip_address,
            last_activity: session.last_activity,
            created_at: session.created_at,
            expires_at: session.expires_at,
        }
    }
}



// Ambil semua active sessions untuk user (multi-device support)
pub async fn get_active_sessions(
    state: &AppState,
    user_id: i32,
) -> Result<Vec<SessionResponse>, AppError> {
    let sessions = UserSession::find_active_by_user(&state.db, user_id).await?;

    let response: Vec<SessionResponse> = sessions.into_iter().map(|s| s.into()).collect();

    Ok(response)
}

// Invalidate session tertentu (logout dari satu device)
pub async fn invalidate_session(
    state: &AppState,
    user_id: i32,
    session_id: i32,
) -> Result<String, AppError> {
    // Cari session
    let session = UserSession::find_by_id(&state.db, session_id)
        .await?
        .ok_or_else(|| AppError::NotFoundError("Session tidak ditemukan".to_string()))?;

    // Verifikasi session milik user yang request
    if session.user_id != user_id {
        return Err(AppError::authorization(
            "Anda tidak memiliki akses untuk session ini"
        ));
    }

    // Invalidate session
    UserSession::invalidate(&state.db, session_id).await?;

    tracing::info!(
        "Session {} invalidated by user_id: {}",
        session_id,
        user_id
    );

    Ok("Session berhasil dihapus".to_string())
}

// Invalidate semua sessions user (logout dari semua device)
pub async fn invalidate_all_sessions(
    state: &AppState,
    user_id: i32,
) -> Result<String, AppError> {
    // Invalidate semua active sessions
    UserSession::invalidate_all_by_user(&state.db, user_id).await?;

    tracing::info!("All sessions invalidated for user_id: {}", user_id);

    Ok("Logout dari semua device berhasil".to_string())
}
