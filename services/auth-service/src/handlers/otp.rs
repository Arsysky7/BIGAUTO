use axum::{
    extract::State,
    http::HeaderMap,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

use crate::{
    config::AppState,
    error::AppResult,
    middleware::auth::extract_authenticated_user,
    models::user::User,
};

/// Response untuk OTP status check
#[derive(Debug, Serialize, ToSchema)]
pub struct OtpStatusResponse {
    #[schema(example = false)]
    pub is_blocked: bool,
    pub blocked_until: Option<DateTime<Utc>>,
    #[schema(example = 15)]
    pub remaining_minutes: Option<i64>,
    #[schema(example = "Akun Anda tidak diblokir. Anda dapat request OTP.")]
    pub message: String,
}


/// Check OTP status for current user
#[utoipa::path(
    get,
    path = "/api/auth/otp-status",
    responses(
        (status = 200, description = "Successfully retrieved OTP status", body = OtpStatusResponse),
    ),
    tag = "Authentication",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn check_otp_status_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl IntoResponse> {
    // Extract user dari JWT token
    let auth_user = extract_authenticated_user(&headers, &state.config.jwt_secret)
        .map_err(|(_status, msg)| crate::error::AppError::authentication(&msg))?;

    // Check OTP block status
    let blocked_until = User::check_otp_blocked(&state.db, auth_user.user_id).await?;

    let now = Utc::now();
    let (is_blocked, remaining_minutes, message) = if let Some(blocked_time) = blocked_until {
        if blocked_time > now {
            // Masih di-block
            let remaining = (blocked_time - now).num_minutes();
            (
                true,
                Some(remaining),
                format!(
                    "Akun Anda diblokir dari request OTP. Silakan coba lagi dalam {} menit.",
                    remaining
                ),
            )
        } else {
            // Block sudah expired
            (
                false,
                None,
                "Akun Anda tidak diblokir. Anda dapat request OTP.".to_string(),
            )
        }
    } else {
        // Tidak pernah di-block
        (
            false,
            None,
            "Akun Anda tidak diblokir. Anda dapat request OTP.".to_string(),
        )
    };

    let response = OtpStatusResponse {
        is_blocked,
        blocked_until,
        remaining_minutes,
        message,
    };

    tracing::info!(
        "User {} checked OTP status. Blocked: {}",
        auth_user.user_id,
        is_blocked
    );

    Ok(Json(response))
}
