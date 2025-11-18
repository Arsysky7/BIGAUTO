use axum::{
    extract::FromRequestParts,
    http::request::Parts,
    RequestPartsExt,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use shared::utils::jwt;

use crate::error::AppError;

// User terautentikasi (any user with valid JWT)
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: i32,
    pub email: String,
    pub role: String,
}

// Seller terautentikasi (hanya seller yang bisa create/update vehicle)
#[derive(Debug, Clone)]
pub struct AuthSeller {
    pub user_id: i32,
    pub email: String,
}

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| AppError::unauthorized("Token tidak ditemukan"))?;

        let claims = jwt::validate_token(bearer.token())
            .map_err(|_| AppError::unauthorized("Token tidak valid atau sudah expired"))?;

        Ok(AuthUser {
            user_id: claims.sub,
            email: claims.email,
            role: claims.role,
        })
    }
}

impl<S> FromRequestParts<S> for AuthSeller
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let user = AuthUser::from_request_parts(parts, _state).await?;

        if user.role != "seller" {
            return Err(AppError::forbidden("Endpoint ini hanya untuk seller"));
        }

        Ok(AuthSeller {
            user_id: user.user_id,
            email: user.email,
        })
    }
}
