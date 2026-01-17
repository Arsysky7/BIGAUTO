use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use sqlx::PgPool;
use std::env;
use thiserror::Error;

// TokenClaims structure khusus untuk User Service
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



#[derive(Debug, Error)]
pub enum JwtError {
    #[error("Token invalid atau expired")]
    InvalidToken,

    #[error("JWT secret tidak ditemukan")]
    MissingSecret,

    #[error("Token type tidak valid untuk endpoint ini")]
    InvalidTokenType,
}

// Validasi JWT signature dan extract claims
fn decode_jwt_token(token: &str) -> Result<TokenClaims, JwtError> {
    let secret = get_jwt_secret()?;
    let validation = Validation::new(Algorithm::HS256);

    let token_data = decode::<TokenClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|_| JwtError::InvalidToken)?;

    Ok(token_data.claims)
}

// Ambil JWT secret dari environment dengan production safety check
fn get_jwt_secret() -> Result<String, JwtError> {
    let secret = env::var("JWT_SECRET")
        .map_err(|_| JwtError::MissingSecret)?;

    // Security check untuk production environment
    if !cfg!(debug_assertions) && secret.contains("change-this") {
        tracing::error!("JWT_SECRET using default value in production!");
        return Err(JwtError::MissingSecret);
    }

    Ok(secret)
}

// Validasi token type (hanya access token yang diperbolehkan)
fn validate_token_type(claims: &TokenClaims) -> Result<(), JwtError> {
    if claims.token_type != "access" {
        tracing::warn!("Invalid token type: {}", claims.token_type);
        return Err(JwtError::InvalidTokenType);
    }
    Ok(())
}

// Cek JWT blacklist di database
async fn check_jwt_blacklist(pool: &PgPool, claims: &TokenClaims) -> Result<(), JwtError> {
    let is_blacklisted: Option<bool> = sqlx::query_scalar::<_, bool>(
        "SELECT is_token_blacklisted_v2($1, $2)"
    )
    .bind(&claims.jti)
    .bind(&claims.token_type)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("JWT blacklist check failed: {}", e);
        JwtError::InvalidToken
    })?;

    if is_blacklisted.unwrap_or(false) {
        tracing::warn!(
            "Blacklisted token access attempt - jti: {}, user_id: {}, email: {}",
            claims.jti,
            claims.sub,
            claims.email
        );
        return Err(JwtError::InvalidToken);
    }

    Ok(())
}

// Main validation function untuk JWT
pub async fn validate_token(token: &str, pool: &PgPool) -> Result<TokenClaims, JwtError> {
    // Decode dan validasi signature
    let claims = decode_jwt_token(token)?;

    // Validasi token type
    validate_token_type(&claims)?;

    // Cek blacklist database
    check_jwt_blacklist(pool, &claims).await?;

    tracing::debug!(
        "JWT validation successful - user_id: {}, email: {}, role: {}",
        claims.sub,
        claims.email,
        claims.role
    );

    Ok(claims)
}


#[cfg(test)]
mod tests {
    // Mock database for testing
    use super::*;

    async fn mock_validate_token_for_tests(token: &str) -> Result<TokenClaims, JwtError> {
        decode_jwt_token(token)
    }
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
    fn test_validate_access_token() {
        std::env::set_var("JWT_SECRET", "test-secret-key-for-testing-only");

        let token = create_test_token(123, "test@example.com", "customer", "access");
        // Skip database validation for unit tests
        let result = decode_jwt_token(&token);

        assert!(result.is_ok());
        let claims = result.unwrap();
        assert_eq!(claims.sub, 123);
        assert_eq!(claims.email, "test@example.com");
        assert_eq!(claims.role, "customer");
        assert_eq!(claims.token_type, "access");
    }

    #[test]
    fn test_reject_refresh_token() {
        std::env::set_var("JWT_SECRET", "test-secret-key-for-testing-only");

        let token = create_test_token(123, "test@example.com", "customer", "refresh");
        // Skip database validation for unit tests
        let result = decode_jwt_token(&token);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), JwtError::InvalidTokenType));
    }

    #[test]
    fn test_missing_secret() {
        std::env::remove_var("JWT_SECRET");

        let token = create_test_token(123, "test@example.com", "customer", "access");
        // Skip database validation for unit tests
        let result = decode_jwt_token(&token);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), JwtError::MissingSecret));
    }

    #[test]
    fn test_seller_token_structure() {
        std::env::set_var("JWT_SECRET", "test-secret-key-for-testing-only");

        let token = create_test_token(456, "seller@example.com", "seller", "access");
        let claims = decode_jwt_token(&token).unwrap();

        // Test akses langsung ke claims 
        assert_eq!(claims.sub, 456);
        assert_eq!(claims.email, "seller@example.com");
        assert_eq!(claims.role, "seller");
        assert_eq!(claims.token_type, "access");

        assert!(claims.role == "seller" || claims.role == "customer");
    }
}