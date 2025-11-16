use axum::http::{HeaderMap, StatusCode};

use crate::{
    handlers::user::AuthenticatedUser,
    utils::jwt,
};

// Helper function untuk ekstrak user dari Authorization header
// Dipakai langsung di handler tanpa perlu custom extractor
pub fn extract_authenticated_user(
    headers: &HeaderMap,
    jwt_secret: &str,
) -> Result<AuthenticatedUser, (StatusCode, String)> {
    // Extract Authorization header
    let auth_header = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                "Authorization header tidak ditemukan. Silakan login.".to_string(),
            )
        })?;

    // Check if it starts with "Bearer "
    if !auth_header.starts_with("Bearer ") {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Format authorization header tidak valid. Gunakan: Bearer <token>".to_string(),
        ));
    }

    // Extract token from "Bearer <token>"
    let token = &auth_header[7..]; // Skip "Bearer "

    // Validate dan decode JWT token menggunakan secret dari AppConfig
    let claims = jwt::validate_token(token, jwt_secret).map_err(|e| {
        (
            StatusCode::UNAUTHORIZED,
            format!("Token tidak valid: {}", e),
        )
    })?;

    // Verify token type is access token (bukan refresh token)
    if claims.token_type != "access" {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Token type tidak valid. Gunakan access token.".to_string(),
        ));
    }

    // Convert role string to is_seller boolean
    let is_seller = claims.role == "seller";

    // Return AuthenticatedUser dengan data dari JWT claims
    Ok(AuthenticatedUser {
        user_id: claims.sub, // JWT uses "sub" for user_id
        email: claims.email,
        is_seller,
    })
}
