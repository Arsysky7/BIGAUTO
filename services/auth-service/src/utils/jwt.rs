use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

/// Structure untuk JWT claims
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenClaims {
    pub sub: i32,
    pub email: String,
    pub role: String,
    pub exp: i64,
    pub iat: i64,
    pub token_type: String,
    pub jti: String,
}

/// Generate access token dengan expiry 15 menit 
pub fn generate_access_token(
    user_id: i32,
    email: &str,
    role: &str,
    jwt_secret: &str,
    jwt_access_expiry: i64,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = Utc::now();
    let exp = now + Duration::seconds(jwt_access_expiry);

    let claims = TokenClaims {
        sub: user_id,
        email: email.to_string(),
        role: role.to_string(),
        exp: exp.timestamp(),
        iat: now.timestamp(),
        token_type: "access".to_string(),
        jti: uuid::Uuid::new_v4().to_string(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes())
    )
}

/// Generate refresh token dengan expiry 7 hari 
pub fn generate_refresh_token(
    user_id: i32,
    email: &str,
    role: &str,
    jwt_secret: &str,
    jwt_refresh_expiry: i64,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = Utc::now();
    let exp = now + Duration::seconds(jwt_refresh_expiry);

    let claims = TokenClaims {
        sub: user_id,
        email: email.to_string(),
        role: role.to_string(),
        exp: exp.timestamp(),
        iat: now.timestamp(),
        token_type: "refresh".to_string(),
        jti: uuid::Uuid::new_v4().to_string(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes())
    )
}

/// Validasi JWT token signature dan extract claims
pub fn validate_token_signature(
    token: &str,
    jwt_secret: &str,
) -> Result<TokenClaims, jsonwebtoken::errors::Error> {
    let mut validation = Validation::default();
    validation.leeway = 60; 
    let token_data = decode::<TokenClaims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &validation,
    )?;

    Ok(token_data.claims)
}

/// Validasi token lengkap dengan signature dan blacklist check
pub async fn validate_token(
    token: &str,
    jwt_secret: &str,
    db: &PgPool,
) -> Result<TokenClaims, String> {
    // Step 1: Validasi signature JWT
    let claims = validate_token_signature(token, jwt_secret)
        .map_err(|e| format!("Token signature invalid: {}", e))?;

    // Step 2: Validasi expiry time
    let now = Utc::now().timestamp();
    if claims.exp < now {
        return Err("Token has expired".to_string());
    }

    // Step 3: Validasi token type (hanya access/refresh)
    if !matches!(claims.token_type.as_str(), "access" | "refresh") {
        return Err("Invalid token type".to_string());
    }

    // Step 4: Validasi blacklist
    validate_blacklist_status(&claims, db).await?;

    Ok(claims)
}

/// Check token status di blacklist 
pub async fn validate_blacklist_status(
    claims: &TokenClaims,
    db: &PgPool,
) -> Result<(), String> {
    let is_blacklisted: bool = sqlx::query_scalar!(
        "SELECT is_token_blacklisted_v2($1, $2)",
        claims.jti,
        claims.token_type
    )
    .fetch_one(db)
    .await
    .map_err(|e| format!("Blacklist validation failed: {}", e))?
    .unwrap_or(false);

    if is_blacklisted {
        tracing::warn!("Token has been blacklisted - user_id: {}, jti: {}...",
            claims.sub,
            &claims.jti[..8.min(claims.jti.len())]
        );
        return Err("Token has been revoked".to_string());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_generate_and_validate_access_token() {
        let user_id = 123;
        let email = "test@example.com";
        let role = "customer";
        let jwt_secret = "test-secret-key";
        let jwt_access_expiry = 900;

        let token = generate_access_token(user_id, email, role, jwt_secret, jwt_access_expiry)
            .expect("Gagal generate access token");

        let claims = validate_token_signature(&token, jwt_secret).expect("Gagal validate token");

        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.email, email);
        assert_eq!(claims.role, role);
        assert_eq!(claims.token_type, "access");
    }

    #[test]
    fn test_generate_and_validate_refresh_token() {
        let user_id = 456;
        let email = "seller@example.com";
        let role = "seller";
        let jwt_secret = "test-secret-key";
        let jwt_refresh_expiry = 604800;

        let token = generate_refresh_token(user_id, email, role, jwt_secret, jwt_refresh_expiry)
            .expect("Gagal generate refresh token");

        let claims = validate_token_signature(&token, jwt_secret).expect("Gagal validate token");

        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.email, email);
        assert_eq!(claims.role, role);
        assert_eq!(claims.token_type, "refresh");
    }

    #[test]
    fn test_invalid_token() {
        let invalid_token = "invalid.jwt.token";
        let result = validate_token_signature(invalid_token, "test-secret");

        assert!(result.is_err(), "Token invalid seharusnya error");
    }

    #[test]
    fn test_token_expiry_timestamp() {
        let token = generate_access_token(1, "test@test.com", "customer", "test-secret", 900)
            .expect("Gagal generate token");

        let claims = validate_token_signature(&token, "test-secret").expect("Gagal validate");
        let now = Utc::now().timestamp();

        assert!(claims.exp > now, "Token expiry harus di masa depan");
        assert!(claims.iat <= now, "Token issued time tidak boleh masa depan");
    }

  }