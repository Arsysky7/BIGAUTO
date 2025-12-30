// JWT-Only Authentication untuk Booking Service
use axum::{
    extract::{Request, State},
    http::HeaderMap,
    response::Response,
    middleware::Next,
};
use crate::{config::AppState, error::AppError, utils::jwt};

// User authentication context 
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: i32,
    pub email: String,
    pub role: String,
}

// Customer authentication context - all users
#[derive(Debug, Clone)]
pub struct AuthCustomer {
    pub user_id: i32,
    pub email: String,
}

// Seller authentication context - role-restricted access
#[derive(Debug, Clone)]
pub struct AuthSeller {
    pub user_id: i32,
}

// Implementasi Axum extractor untuk AuthUser 
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

// Implementasi Axum extractor untuk AuthCustomer (all users)
impl<S> axum::extract::FromRequestParts<S> for AuthCustomer
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts<'life0, 'life1>(
        parts: &'life0 mut axum::http::request::Parts,
        state: &'life1 S,
    ) -> Result<Self, Self::Rejection> {
        let auth_user = AuthUser::from_request_parts(parts, state).await?;

        Ok(AuthCustomer {
            user_id: auth_user.user_id,
            email: auth_user.email,
        })
    }
}

// Implementasi Axum extractor untuk AuthSeller
impl<S> axum::extract::FromRequestParts<S> for AuthSeller
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts<'life0, 'life1>(
        parts: &'life0 mut axum::http::request::Parts,
        state: &'life1 S,
    ) -> Result<Self, Self::Rejection> {
        let auth_user = AuthUser::from_request_parts(parts, state).await?;

        if auth_user.role != "seller" {
            return Err(AppError::forbidden("Seller authentication required"));
        }

        Ok(AuthSeller {
            user_id: auth_user.user_id,
        })
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

// JWT Authentication middleware dengan database blacklist validation
pub async fn jwt_auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Extract path before using request
    let path = request.uri().path().to_string();
    if path == "/health" || path == "/info" {
        return Ok(next.run(request).await);
    }

    // Extract JWT token dari Authorization header
    let headers = request.headers();
    let token = extract_jwt_token(headers)?;

    // Validasi JWT dengan database trust boundary
    let claims = jwt::validate_token(&token, &state.db)
        .await
        .map_err(|_| AppError::unauthorized("Token tidak valid, expired, atau sudah di-blacklist"))?;

    // Prepare user data untuk injection
    let auth_user = AuthUser {
        user_id: claims.sub,
        email: claims.email.clone(),
        role: claims.role.clone(),
    };

    // Inject user data ke request extensions untuk extractor handlers
    request.extensions_mut().insert(auth_user.clone());

    // Security audit log
    tracing::debug!(
        "User authenticated successfully - ID: {}, Email: {}, Role: {}, Endpoint: {}",
        auth_user.user_id,
        auth_user.email,
        auth_user.role,
        path
    );

    Ok(next.run(request).await)
}


