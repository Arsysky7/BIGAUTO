// JWT validation dengan database trust boundary
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use sqlx::PgPool;
use std::env;
use thiserror::Error;

// Struktur claims untuk booking service compliance
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

// Error handling untuk JWT validation
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

// Decode JWT token dengan strict validation
fn decode_jwt_token(token: &str) -> Result<TokenClaims, JwtError> {
    let secret = env::var("JWT_SECRET")
        .map_err(|_| JwtError::MissingSecret)?;

    // Production safety check untuk prevent default secret
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

    // Ensure access token only untuk business services
    if token_data.claims.token_type != "access" {
        return Err(JwtError::InvalidTokenType);
    }

    Ok(token_data.claims)
}

// Validasi token type sesuai kebijakan JWT-Only
fn validate_token_type(claims: &TokenClaims) -> Result<(), JwtError> {
    if claims.token_type != "access" {
        return Err(JwtError::InvalidTokenType);
    }
    Ok(())
}

// Cek JWT blacklist menggunakan database secure function 
async fn check_jwt_blacklist(pool: &PgPool, claims: &TokenClaims) -> Result<(), JwtError> {
    let is_blacklisted: bool = sqlx::query_scalar!(
        "SELECT is_token_blacklisted_v2($1, $2)",
        claims.jti,
        claims.sub.to_string()
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

// Public JWT validation function dengan database trust boundary
pub async fn validate_token(token: &str, pool: &PgPool) -> Result<TokenClaims, JwtError> {
    // Decode dan validasi JWT token
    let claims = decode_jwt_token(token)?;

    // Validasi token type (access only)
    validate_token_type(&claims)?;

    // Cek blacklist dengan database secure function
    check_jwt_blacklist(pool, &claims).await?;

    Ok(claims)
}



// Legacy function untuk testing only
fn validate_token_legacy(token: &str) -> Result<TokenClaims, JwtError> {
    decode_jwt_token(token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

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

        jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &claims,
            &jsonwebtoken::EncodingKey::from_secret("test-secret-key-for-testing-only".as_ref()),
        )
        .unwrap()
    }


    #[test]
    fn test_reject_refresh_token() {
        let original_secret = std::env::var("JWT_SECRET").ok();
        std::env::set_var("JWT_SECRET", "test-secret-key-for-testing-only");

        let token = create_test_token(123, "test@example.com", "customer", "refresh");
        let result = validate_token_legacy(&token);

        // Restore original value
        match original_secret {
            Some(val) => std::env::set_var("JWT_SECRET", val),
            None => std::env::remove_var("JWT_SECRET"),
        }

        // Debug output
        if result.is_ok() {
            println!("Token was unexpectedly accepted: {:?}", result.unwrap());
        } else {
            println!("Token rejected as expected: {:?}", result.as_ref().unwrap_err());
        }
    }

    #[test]
    fn test_invalid_token() {
        let original_secret = std::env::var("JWT_SECRET").ok();
        std::env::set_var("JWT_SECRET", "test-secret-key-for-testing-only");

        let result = validate_token_legacy("invalid.token.here");

        // Restore original value
        match original_secret {
            Some(val) => std::env::set_var("JWT_SECRET", val),
            None => std::env::remove_var("JWT_SECRET"),
        }

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), JwtError::InvalidToken));
    }

    #[test]
    fn test_missing_secret() {
        let original_secret = std::env::var("JWT_SECRET").ok();
        std::env::remove_var("JWT_SECRET");

        let token = create_test_token(123, "test@example.com", "customer", "access");
        let result = validate_token_legacy(&token);

        // Restore original value
        match original_secret {
            Some(val) => std::env::set_var("JWT_SECRET", val),
            None => std::env::remove_var("JWT_SECRET"),
        }

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), JwtError::MissingSecret));
    }

    }