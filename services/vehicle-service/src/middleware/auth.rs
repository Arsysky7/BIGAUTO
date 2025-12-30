// JWT-Only Authentication Middleware for Vehicle Service

use axum::{
    extract::{Request, State},
    http::HeaderMap,
    response::Response,
    middleware::Next,
};
use crate::{config::AppState, error::AppError};

// Import JWT validation from utils
use crate::utils::jwt;

// User yang sudah terautentikasi
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: i32,
    pub email: String,
    pub role: String,
}

// Implement Axum extractor untuk AuthUser
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

// Seller terautentikasi
#[derive(Debug, Clone)]
pub struct AuthSeller {
    pub user_id: i32,
    pub email: String,
}

// Implement Axum extractor untuk AuthSeller
impl<S> axum::extract::FromRequestParts<S> for AuthSeller
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts<'life0, 'life1>(
        parts: &'life0 mut axum::http::request::Parts,
        _state: &'life1 S,
    ) -> Result<Self, Self::Rejection> {
        // Cari AuthUser yang sudah di-inject 
        let auth_user = parts
            .extensions
            .get::<AuthUser>()
            .ok_or_else(|| AppError::unauthorized("Authentication required"))?;

        // Validasi role seller
        if auth_user.role != "seller" {
            return Err(AppError::forbidden("Seller authentication required"));
        }

        Ok(AuthSeller {
            user_id: auth_user.user_id,
            email: auth_user.email.clone(),
        })
    }
}

// Extract JWT token dari Authorization header
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

// Authentication middleware dengan JWT blacklist check
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Extract JWT token
    let token = extract_jwt_token(request.headers())?;

    // Validasi JWT dengan blacklist check
    let claims = jwt::validate_token(&token, &state.db)
        .await
        .map_err(|_| AppError::unauthorized("Invalid or expired JWT token"))?;

    // Clone claims untuk multiple uses
    let user_id = claims.sub;
    let email = claims.email.clone();
    let role = claims.role.clone();

    // Inject user data ke request extensions
    request.extensions_mut().insert(AuthUser {
        user_id,
        email: email.clone(),
        role: role.clone(),
    });

    // Log untuk audit trail
    tracing::debug!(
        "User authenticated - id: {}, email: {}, role: {}",
        user_id,
        email,
        role
    );

    Ok(next.run(request).await)
}
