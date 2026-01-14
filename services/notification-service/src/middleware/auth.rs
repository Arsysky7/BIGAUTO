// JWT-Only Authentication Middleware untuk Notification Service

use axum::{
    extract::{Request, State},
    http::HeaderMap,
    response::Response,
    middleware::Next,
};
use crate::{config::AppState, error::AppError};

// Import JWT validation dari utils
use crate::utils::jwt;

/// User yang sudah terautentikasi
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: i32,
    pub role: String,  
}

/// Implement Axum extractor untuk AuthUser
impl<S> axum::extract::FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts<'life0, 'life1>(
        parts: &'life0 mut axum::http::request::Parts,
        _state: &'life1 S,
    ) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthUser>()
            .cloned()
            .ok_or_else(|| AppError::unauthorized("Authentication required"))
    }
}

/// Extract JWT token dari Authorization header
fn extract_jwt_token(headers: &HeaderMap) -> Result<String, AppError> {
    let auth_header = headers
        .get("authorization")
        .ok_or_else(|| AppError::unauthorized("Authorization header missing"))?
        .to_str()
        .map_err(|_| AppError::unauthorized("Invalid authorization header"))?;

    if !auth_header.starts_with("Bearer ") {
        return Err(AppError::unauthorized("Invalid authorization header format"));
    }

    Ok(auth_header[7..].to_string())
}

/// Authentication middleware dengan JWT blacklist
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Extract JWT token
    let token = extract_jwt_token(request.headers())?;

    // Validasi JWT dengan blacklist
    let claims = jwt::validate_token(&token, &state.db)
        .await
        .map_err(|_| AppError::unauthorized("Invalid or expired JWT token"))?;

    // Inject user data ke request extensions
    request.extensions_mut().insert(AuthUser {
        user_id: claims.sub,
        role: claims.role.clone(),
    });

    // Log untuk audit trail
    tracing::debug!(
        "User authenticated - id: {}, email: {}, role: {}",
        claims.sub,
        claims.email,
        claims.role
    );

    Ok(next.run(request).await)
}