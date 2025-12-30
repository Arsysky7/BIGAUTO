// JWT-Only Authentication Middleware untuk Payment Service

use axum::{
    extract::{Request, State},
    http::HeaderMap,
    response::Response,
    middleware::Next,
};
use crate::{config::AppState, error::AppError, utils::jwt};

// Authentication context untuk user yang sudah terautentikasi
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: i32,
    pub email: String,
    pub role: String,
}

// Axum extractor implementation untuk AuthUser
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

// Extract Bearer token dari Authorization header
fn extract_jwt_token(headers: &HeaderMap) -> Result<String, AppError> {
    let auth_header = headers
        .get("authorization")
        .ok_or_else(|| AppError::unauthorized("Authorization header dengan Bearer token diperlukan"))?
        .to_str()
        .map_err(|_| AppError::unauthorized("Invalid Authorization header format"))?;

    if !auth_header.starts_with("Bearer ") {
        return Err(AppError::unauthorized("Bearer token format diperlukan"));
    }

    Ok(auth_header[7..].to_string())
}

// JWT authentication middleware dengan blacklist validation
pub async fn jwt_auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Skip authentication untuk health check dan webhooks
    let path = request.uri().path().to_string();
    if path == "/health" || path == "/info" || path.contains("webhooks") {
        return Ok(next.run(request).await);
    }

    // Extract JWT token dari Authorization header
    let token = extract_jwt_token(request.headers())?;

    // Validasi JWT dengan database trust boundary
    let claims = jwt::validate_token(&token, &state.db)
        .await
        .map_err(|_| AppError::unauthorized("Token tidak valid, expired, atau sudah di-blacklist"))?;

    // Prepare user context untuk handlers
    let auth_user = AuthUser {
        user_id: claims.sub,
        email: claims.email.clone(),
        role: claims.role.clone(),
    };

    // Inject ke request extensions agar bisa di-extract oleh handlers
    request.extensions_mut().insert(auth_user.clone());

    // Security audit log
    tracing::debug!(
        "User authenticated - ID: {}, Email: {}, Role: {}, Endpoint: {}",
        auth_user.user_id,
        auth_user.email,
        auth_user.role,
        path
    );

    Ok(next.run(request).await)
}