// JWT validation dengan database untuk Payment Service

use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use sqlx::PgPool;
use std::env;
use thiserror::Error;

// Claims structure untuk JWT token
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TokenClaims {
    pub sub: i32,
    pub email: String,
    pub role: String,
    pub exp: i64,
    pub iat: i64,
    pub token_type: String,
    pub jti: String,
}

// Error types untuk JWT validation
#[derive(Debug, Error)]
pub enum JwtError {
    #[error("Token invalid atau expired")]
    InvalidToken,
    #[error("JWT secret tidak ditemukan")]
    MissingSecret,
    #[error("Token type tidak valid untuk endpoint ini")]
    InvalidTokenType,
    #[error("Token sudah di-blacklist")]
    TokenBlacklisted,
    #[error("Database error saat validasi blacklist")]
    DatabaseError,
}

// Decode JWT token dan validasi signature
fn decode_jwt_token(token: &str) -> Result<TokenClaims, JwtError> {
    let secret = env::var("JWT_SECRET")
        .map_err(|_| JwtError::MissingSecret)?;

    // Production safety check
    if !cfg!(debug_assertions) && secret.contains("change-this") {
        return Err(JwtError::MissingSecret);
    }

    let validation = Validation::new(Algorithm::HS256);
    let token_data = decode::<TokenClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|_| JwtError::InvalidToken)?;

    // Business services hanya terima access token
    if token_data.claims.token_type != "access" {
        return Err(JwtError::InvalidTokenType);
    }

    Ok(token_data.claims)
}

// Cek apakah token sudah di-blacklist menggunakan secure function
async fn check_jwt_blacklist(pool: &PgPool, claims: &TokenClaims) -> Result<(), JwtError> {
    let is_blacklisted: bool = sqlx::query_scalar!(
        "SELECT is_token_blacklisted_v2($1, $2)",
        claims.jti,
        claims.token_type
    )
    .fetch_one(pool)
    .await
    .map_err(|_| JwtError::DatabaseError)?
    .unwrap_or(true);

    if is_blacklisted {
        return Err(JwtError::TokenBlacklisted);
    }

    Ok(())
}

// Main validation function dengan database t
pub async fn validate_token(token: &str, pool: &PgPool) -> Result<TokenClaims, JwtError> {
    // Decode dan validasi signature
    let claims = decode_jwt_token(token)?;

    // Cek blacklist menggunakan secure function 
    check_jwt_blacklist(pool, &claims).await?;

    Ok(claims)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use jsonwebtoken::{encode, EncodingKey, Header};

    fn create_test_token(user_id: i32, email: &str, role: &str, token_type: &str) -> String {
        let now = Utc::now();
        let claims = TokenClaims {
            sub: user_id,
            email: email.to_string(),
            role: role.to_string(),
            exp: (now + Duration::minutes(15)).timestamp(),
            iat: now.timestamp(),
            token_type: token_type.to_string(),
            jti: "test-jti-123".to_string(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret("test-secret-key-for-testing-only".as_ref()),
        )
        .unwrap()
    }

    #[test]
    fn test_decode_access_token_success() {
        std::env::set_var("JWT_SECRET", "test-secret-key-for-testing-only");

        let token = create_test_token(123, "test@example.com", "customer", "access");
        let result = decode_jwt_token(&token);

        assert!(result.is_ok());
        let claims = result.unwrap();
        assert_eq!(claims.sub, 123);
        assert_eq!(claims.token_type, "access");
    }

    #[test]
    fn test_decode_reject_refresh_token() {
        std::env::set_var("JWT_SECRET", "test-secret-key-for-testing-only");

        let token = create_test_token(123, "test@example.com", "customer", "refresh");
        let result = decode_jwt_token(&token);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), JwtError::InvalidTokenType));
    }

    #[test]
    fn test_decode_invalid_token_format() {
        std::env::set_var("JWT_SECRET", "test-secret-key-for-testing-only");

        let result = decode_jwt_token("invalid.token.here");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), JwtError::InvalidToken));
    }

    #[test]
    fn test_missing_secret_environment() {
        std::env::remove_var("JWT_SECRET");

        let token = create_test_token(123, "test@example.com", "customer", "access");
        let result = decode_jwt_token(&token);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), JwtError::MissingSecret));
    }
}