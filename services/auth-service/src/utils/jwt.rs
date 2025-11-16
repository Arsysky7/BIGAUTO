use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use shared::models::claims::TokenClaims;
use uuid::Uuid;

// Generate access token dengan JTI untuk tracking (default 15 menit sesuai requirement)
pub fn generate_access_token(
    user_id: i32,
    email: &str,
    role: &str,
    jwt_secret: &str,
    expiry_seconds: i64,
) -> Result<(String, String), jsonwebtoken::errors::Error> {
    let now = Utc::now();
    let exp = now + Duration::seconds(expiry_seconds);

    // Generate unique JTI for this access token
    let jti = Uuid::new_v4().to_string();

    let claims = TokenClaims {
        sub: user_id,
        email: email.to_string(),
        role: role.to_string(),
        exp: exp.timestamp(),
        iat: now.timestamp(),
        token_type: "access".to_string(),
        jti: jti.clone(),
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes())
    )?;

    // Return both token and JTI
    Ok((token, jti))
}

// Generate refresh token dengan JTI (default 7 hari sesuai requirement)
pub fn generate_refresh_token(
    user_id: i32,
    email: &str,
    role: &str,
    jwt_secret: &str,
    expiry_seconds: i64,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = Utc::now();
    let exp = now + Duration::seconds(expiry_seconds);

    // Generate unique JTI for refresh token (stored in user_sessions.refresh_token)
    let jti = Uuid::new_v4().to_string();

    let claims = TokenClaims {
        sub: user_id,
        email: email.to_string(),
        role: role.to_string(),
        exp: exp.timestamp(),
        iat: now.timestamp(),
        token_type: "refresh".to_string(),
        jti,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes())
    )
}

// Validasi token dan extract claims jika valid
pub fn validate_token(token: &str, jwt_secret: &str) -> Result<TokenClaims, jsonwebtoken::errors::Error> {
    let validation = Validation::default();

    let token_data = decode::<TokenClaims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &validation,
    )?;

    Ok(token_data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SECRET: &str = "test-secret-key-for-unit-tests";
    const TEST_ACCESS_EXPIRY: i64 = 900; // 15 minutes
    const TEST_REFRESH_EXPIRY: i64 = 604800; // 7 days

    #[test]
    fn test_generate_and_validate_access_token() {
        let user_id = 123;
        let email = "test@example.com";
        let role = "customer";

        let (token, jti) = generate_access_token(user_id, email, role, TEST_SECRET, TEST_ACCESS_EXPIRY)
            .expect("Gagal generate access token");

        let claims = validate_token(&token, TEST_SECRET).expect("Gagal validate token");

        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.email, email);
        assert_eq!(claims.role, role);
        assert_eq!(claims.token_type, "access");
        assert_eq!(claims.jti, jti, "JTI harus match");
        assert!(!jti.is_empty(), "JTI tidak boleh kosong");
    }

    #[test]
    fn test_generate_and_validate_refresh_token() {
        let user_id = 456;
        let email = "seller@example.com";
        let role = "seller";

        let token = generate_refresh_token(user_id, email, role, TEST_SECRET, TEST_REFRESH_EXPIRY)
            .expect("Gagal generate refresh token");

        let claims = validate_token(&token, TEST_SECRET).expect("Gagal validate token");

        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.email, email);
        assert_eq!(claims.role, role);
        assert_eq!(claims.token_type, "refresh");
        assert!(!claims.jti.is_empty(), "Refresh token JTI tidak boleh kosong");
    }

    #[test]
    fn test_invalid_token() {
        let invalid_token = "invalid.jwt.token";
        let result = validate_token(invalid_token, TEST_SECRET);

        assert!(result.is_err(), "Token invalid seharusnya error");
    }

    #[test]
    fn test_token_expiry_timestamp() {
        let (token, _jti) = generate_access_token(1, "test@test.com", "customer", TEST_SECRET, TEST_ACCESS_EXPIRY)
            .expect("Gagal generate token");

        let claims = validate_token(&token, TEST_SECRET).expect("Gagal validate");
        let now = Utc::now().timestamp();

        assert!(claims.exp > now, "Token expiry harus di masa depan");
        assert!(claims.iat <= now, "Token issued time tidak boleh masa depan");
    }

    #[test]
    fn test_jti_uniqueness() {
        // Generate 3 tokens untuk user yang sama
        let (token1, jti1) = generate_access_token(1, "test@test.com", "customer", TEST_SECRET, TEST_ACCESS_EXPIRY)
            .expect("Token 1");
        let (token2, jti2) = generate_access_token(1, "test@test.com", "customer", TEST_SECRET, TEST_ACCESS_EXPIRY)
            .expect("Token 2");
        let (token3, jti3) = generate_access_token(1, "test@test.com", "customer", TEST_SECRET, TEST_ACCESS_EXPIRY)
            .expect("Token 3");

        // JTI harus unique
        assert_ne!(jti1, jti2, "JTI 1 dan 2 harus berbeda");
        assert_ne!(jti2, jti3, "JTI 2 dan 3 harus berbeda");
        assert_ne!(jti1, jti3, "JTI 1 dan 3 harus berbeda");

        // Token harus berbeda
        assert_ne!(token1, token2, "Token 1 dan 2 harus berbeda");
        assert_ne!(token2, token3, "Token 2 dan 3 harus berbeda");
    }
}