// Authentication middleware untuk axum handlers
use axum::{
    extract::{Request, FromRequestParts},
    http::{request::Parts, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::{
    models::claims::TokenClaims,
    utils::jwt::validate_token,
};

// Generic AppState trait untuk auth middleware
pub trait HasAuth: Clone {
    fn clone(&self) -> Self;
}

// Context untuk authenticated user
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub claims: TokenClaims,
}

#[derive(Debug, Clone)]
pub struct AuthCustomer {
    pub claims: TokenClaims,
}

#[derive(Debug, Clone)]
pub struct AuthSeller {
    pub claims: TokenClaims,
}

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| {
                (StatusCode::UNAUTHORIZED, Json(json!({
                    "error": "Authorization header required"
                })))
                    .into_response()
            })?;

        if !auth_header.starts_with("Bearer ") {
            return Err((StatusCode::UNAUTHORIZED, Json(json!({
                "error": "Invalid authorization format"
            })))
                .into_response());
        }

        let token = &auth_header[7..];
        let claims = validate_token(token).map_err(|_| {
            (StatusCode::UNAUTHORIZED, Json(json!({
                "error": "Invalid or expired token"
            })))
                .into_response()
        })?;

        Ok(AuthUser { claims })
    }
}

impl<S> FromRequestParts<S> for AuthCustomer
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth_user = AuthUser::from_request_parts(parts, _state).await?;

        if !auth_user.claims.is_customer() {
            return Err((StatusCode::FORBIDDEN, Json(json!({
                "error": "Customer access required"
            })))
                .into_response());
        }

        Ok(AuthCustomer { claims: auth_user.claims })
    }
}

impl<S> FromRequestParts<S> for AuthSeller
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth_user = AuthUser::from_request_parts(parts, _state).await?;

        if !auth_user.claims.is_seller() {
            return Err((StatusCode::FORBIDDEN, Json(json!({
                "error": "Seller access required"
            })))
                .into_response());
        }

        Ok(AuthSeller { claims: auth_user.claims })
    }
}

// Middleware untuk optional authentication (guest access)
pub async fn optional_auth_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    Ok(next.run(request).await)
}