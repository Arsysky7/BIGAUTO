use axum::http::{HeaderMap, StatusCode};
use sqlx::PgPool;

use crate::{
    handlers::user::AuthenticatedUser,
    models::user::User,
    utils::jwt::validate_token,
};

/// Ekstrak dan validasi JWT token dari Authorization header
pub async fn extract_authenticated_user(
    headers: &HeaderMap,
    jwt_secret: &str,
    db: &PgPool,
) -> Result<AuthenticatedUser, (StatusCode, String)> {
    // Ekstrak Bearer token menggunakan shared library
    let auth_header = shared::utils::token_extraction::extract_auth_header(headers)
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Authorization header required".to_string()))?;

    let token = shared::utils::token_extraction::extract_bearer_token(&auth_header)
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Bearer token required".to_string()))?;

    // Validasi format token untuk security
    if token.len() < 50 {
        return Err((StatusCode::UNAUTHORIZED, "Token too short".to_string()));
    }

    if token.contains(' ') || token.contains('\n') || token.contains('\r') {
        return Err((StatusCode::UNAUTHORIZED, "Token contains invalid characters".to_string()));
    }

    // Validasi JWT signature dan blacklist check
    let claims = validate_token(&token, jwt_secret, db)
        .await
        .map_err(|msg| (StatusCode::UNAUTHORIZED, msg))?;

    // Load user untuk dapatkan role hybrid
    let user = User::find_by_id(db, claims.sub)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to load user data".to_string()))?
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "User not found".to_string()))?;

    // Create authenticated user data dengan clone untuk avoid move
    let auth_user = AuthenticatedUser {
        user_id: user.id,
        email: user.email.clone(),
        is_customer: user.is_customer(),
        is_seller: user.is_seller_role(),
    };

    tracing::debug!("Successfully authenticated user: {} ({}) - roles: customer={}, seller={}",
        auth_user.email, auth_user.user_id, auth_user.is_customer, auth_user.is_seller);

    Ok(auth_user)
}