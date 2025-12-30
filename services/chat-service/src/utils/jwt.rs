// JWT validation dengan database trust boundary untuk Chat Service

use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use sqlx::PgPool;
use std::env;
use thiserror::Error;

// TokenClaims structure untuk Chat Service
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

impl TokenClaims {
    // Helper functions untuk role validation
    pub fn is_customer(&self) -> bool {
        self.role == "customer"
    }

    pub fn is_seller(&self) -> bool {
        self.role == "seller"
    }

    pub fn can_access_chat(&self) -> bool {
        self.is_customer() || self.is_seller()
    }
}

#[derive(Debug, Error)]
pub enum JwtError {
    #[error("Token invalid atau expired")]
    InvalidToken,

    #[error("JWT secret tidak ditemukan")]
    MissingSecret,

    #[error("Token type tidak valid untuk endpoint ini")]
    InvalidTokenType,

    #[error("Role tidak valid untuk chat service")]
    InvalidRole,

    #[error("JWT secret menggunakan default value")]
    InsecureSecret,

    #[error("Token sudah di-blacklist")]
    TokenBlacklisted,

    #[error("Database error")]
    DatabaseError,
}

// Decode JWT token dan validasi signature
fn decode_jwt_token(token: &str) -> Result<TokenClaims, JwtError> {
    let secret = env::var("JWT_SECRET")
        .map_err(|_| JwtError::MissingSecret)?;

    // Production safety check - tidak boleh pakai default value
    if !cfg!(debug_assertions) && secret.contains("change-this") {
        return Err(JwtError::InsecureSecret);
    }

    let validation = Validation::new(Algorithm::HS256);

    let token_data = decode::<TokenClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|_| JwtError::InvalidToken)?;

    // Pastikan token adalah access token, bukan refresh token
    if token_data.claims.token_type != "access" {
        return Err(JwtError::InvalidTokenType);
    }

    Ok(token_data.claims)
}

// Cek JWT blacklist menggunakan database secure function
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

// Validasi JWT token dengan database trust boundary (SYNC wrapper)
// Untuk WebSocket handler yang butuh sync validation
pub fn validate_token(token: &str) -> Result<TokenClaims, JwtError> {
    let claims = decode_jwt_token(token)?;

    // Validasi role untuk chat service
    if !claims.can_access_chat() {
        return Err(JwtError::InvalidRole);
    }

    tracing::debug!("JWT validation successful for user: {} ({})", claims.sub, claims.email);

    Ok(claims)
}

// Validasi JWT token dengan database trust boundary (ASYNC full check)
// Untuk HTTP request handlers yang perlu blacklist check
pub async fn validate_token_with_blacklist(
    token: &str,
    pool: &PgPool,
) -> Result<TokenClaims, JwtError> {
    let claims = decode_jwt_token(token)?;

    // Validasi role untuk chat service
    if !claims.can_access_chat() {
        return Err(JwtError::InvalidRole);
    }

    // Cek blacklist dengan database trust boundary
    check_jwt_blacklist(pool, &claims).await?;

    tracing::debug!("JWT validation successful (with blacklist check) for user: {} ({})", claims.sub, claims.email);

    Ok(claims)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Utc, Duration};

    fn create_test_token(user_id: i32, email: &str, role: &str) -> String {
        let claims = TokenClaims {
            sub: user_id,
            email: email.to_string(),
            role: role.to_string(),
            exp: (Utc::now() + Duration::hours(1)).timestamp(),
            iat: Utc::now().timestamp(),
            token_type: "access".to_string(),
            jti: "test-jti".to_string(),
        };

        jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &claims,
            &jsonwebtoken::EncodingKey::from_secret("test-secret-key-for-testing".as_bytes()),
        )
        .unwrap()
    }

    #[test]
    fn test_validate_customer_token() {
        let token = create_test_token(1, "customer@test.com", "customer");

        // Set environment variable untuk testing
        std::env::set_var("JWT_SECRET", "test-secret-key-for-testing");

        let result = validate_token(&token);
        assert!(result.is_ok());

        let claims = result.unwrap();
        assert_eq!(claims.sub, 1);
        assert_eq!(claims.email, "customer@test.com");
        assert!(claims.is_customer());
        assert!(!claims.is_seller());
        assert!(claims.can_access_chat());
    }

    #[test]
    fn test_validate_seller_token() {
        let token = create_test_token(2, "seller@test.com", "seller");

        std::env::set_var("JWT_SECRET", "test-secret-key-for-testing");

        let result = validate_token(&token);
        assert!(result.is_ok());

        let claims = result.unwrap();
        assert_eq!(claims.sub, 2);
        assert_eq!(claims.email, "seller@test.com");
        assert!(!claims.is_customer());
        assert!(claims.is_seller());
        assert!(claims.can_access_chat());
    }

    #[test]
    fn test_invalid_role_token() {
        let token = create_test_token(3, "admin@test.com", "admin");

        std::env::set_var("JWT_SECRET", "test-secret-key-for-testing");

        let result = validate_token(&token);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), JwtError::InvalidRole));
    }

    #[test]
    fn test_invalid_token_type() {
        let claims = TokenClaims {
            sub: 1,
            email: "test@test.com".to_string(),
            role: "customer".to_string(),
            exp: (Utc::now() + Duration::hours(1)).timestamp(),
            iat: Utc::now().timestamp(),
            token_type: "refresh".to_string(),
            jti: "test-jti".to_string(),
        };

        let token = jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &claims,
            &jsonwebtoken::EncodingKey::from_secret("test-secret-key-for-testing".as_bytes()),
        )
        .unwrap();

        std::env::set_var("JWT_SECRET", "test-secret-key-for-testing");

        let result = validate_token(&token);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), JwtError::InvalidTokenType));
    }
}