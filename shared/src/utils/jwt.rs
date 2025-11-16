use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use std::env;
use thiserror::Error;

use crate::models::claims::TokenClaims;

#[derive(Debug, Error)]
pub enum JwtError {
    #[error("Token invalid atau expired")]
    InvalidToken,

    #[error("JWT secret tidak ditemukan")]
    MissingSecret,

    #[error("Claims tidak valid: {0}")]
    InvalidClaims(String),
}

/// Validate JWT token dan extract claims
pub fn validate_token(token: &str) -> Result<TokenClaims, JwtError> {
    let secret = env::var("JWT_SECRET").map_err(|_| JwtError::MissingSecret)?;

    let validation = Validation::new(Algorithm::HS256);

    let token_data = decode::<TokenClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|_| JwtError::InvalidToken)?;

    Ok(token_data.claims)
}

/// Extract user_id dari token
pub fn extract_user_id(token: &str) -> Result<i32, JwtError> {
    let claims = validate_token(token)?;
    Ok(claims.sub)
}

/// Extract email dari token
pub fn extract_email(token: &str) -> Result<String, JwtError> {
    let claims = validate_token(token)?;
    Ok(claims.email)
}

/// Check apakah user adalah seller
pub fn is_seller(token: &str) -> Result<bool, JwtError> {
    let claims = validate_token(token)?;
    Ok(claims.is_seller())
}

/// Check apakah user adalah customer
pub fn is_customer(token: &str) -> Result<bool, JwtError> {
    let claims = validate_token(token)?;
    Ok(claims.is_customer())
}

/// Extract bearer token dari Authorization header
pub fn extract_bearer_token(auth_header: &str) -> Option<String> {
    if auth_header.starts_with("Bearer ") {
        Some(auth_header[7..].to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_bearer_token() {
        let header = "Bearer abc123token";
        assert_eq!(
            extract_bearer_token(header),
            Some("abc123token".to_string())
        );

        let invalid = "Token abc123";
        assert_eq!(extract_bearer_token(invalid), None);
    }
}
