// JWT Authentication Middleware untuk Financial Service 
use axum::{
    extract::{Request, State},
    http::HeaderMap,
    response::Response,
    middleware::Next,
};
use crate::{config::AppState, error::AppError};

// Import JWT validation dari utils
use crate::utils::jwt;

// User yang sudah terautentikasi
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: i32,
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
            .ok_or_else(|| AppError::AuthenticationError("Authentication required".to_string()))
    }
}

// Seller terautentikasi (khusus endpoint keuangan seller)
#[derive(Debug, Clone)]
pub struct AuthSeller {
    pub user_id: i32,
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
        let auth_user = parts
            .extensions
            .get::<AuthUser>()
            .ok_or_else(|| AppError::AuthenticationError("Authentication required".to_string()))?;

        if auth_user.role != "seller" {
            return Err(AppError::AuthorizationError("Seller access required".to_string()));
        }

        Ok(AuthSeller {
            user_id: auth_user.user_id,
        })
    }
}

// Extract JWT token dari Authorization header
fn extract_jwt_token(headers: &HeaderMap) -> Result<String, AppError> {
    let auth_header = headers
        .get("authorization")
        .ok_or_else(|| AppError::AuthenticationError("Authorization header missing".to_string()))?
        .to_str()
        .map_err(|_| AppError::AuthenticationError("Invalid authorization header".to_string()))?;

    if !auth_header.starts_with("Bearer ") {
        return Err(AppError::AuthenticationError("Invalid authorization header format".to_string()));
    }

    Ok(auth_header[7..].to_string())
}

// Authentication middleware dengan secure blacklist check
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = extract_jwt_token(request.headers())?;

    let claims = jwt::validate_token(&token, &state.db)
        .await
        .map_err(|_| AppError::TokenError("Invalid or expired JWT token".to_string()))?;

    let user_id = claims.sub;
    let email = claims.email.clone();
    let role = claims.role.clone();

    request.extensions_mut().insert(AuthUser {
        user_id,
        role: role.clone(),
    });

    tracing::debug!(
        "User authenticated - id: {}, email: {}, role: {}",
        user_id,
        email,
        role
    );

    Ok(next.run(request).await)
}