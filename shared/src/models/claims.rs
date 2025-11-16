use serde::{Deserialize, Serialize};

/// Model JWT claims yang digunakan di seluruh sistem untuk authentication
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

impl TokenClaims {
    /// Cek apakah token adalah access token
    pub fn is_access_token(&self) -> bool {
        self.token_type == "access"
    }

    /// Cek apakah token adalah refresh token
    pub fn is_refresh_token(&self) -> bool {
        self.token_type == "refresh"
    }

    /// Cek apakah user memiliki role seller
    pub fn is_seller(&self) -> bool {
        self.role == "seller"
    }

    /// Cek apakah user memiliki role customer
    pub fn is_customer(&self) -> bool {
        self.role == "customer"
    }

    /// Cek apakah token sudah expired berdasarkan current time
    pub fn is_expired(&self) -> bool {
        let now = chrono::Utc::now().timestamp();
        self.exp <= now
    }

    /// Get remaining validity duration dalam detik
    pub fn remaining_validity(&self) -> i64 {
        let now = chrono::Utc::now().timestamp();
        (self.exp - now).max(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_claims() -> TokenClaims {
        let now = chrono::Utc::now().timestamp();
        TokenClaims {
            sub: 123,
            email: "test@example.com".to_string(),
            role: "customer".to_string(),
            exp: now + 900,
            iat: now,
            token_type: "access".to_string(),
            jti: "unique-jti-123".to_string(),
        }
    }

    #[test]
    fn test_is_access_token() {
        let claims = create_test_claims();
        assert!(claims.is_access_token());
        assert!(!claims.is_refresh_token());
    }

    #[test]
    fn test_is_seller() {
        let mut claims = create_test_claims();
        claims.role = "seller".to_string();
        assert!(claims.is_seller());
        assert!(!claims.is_customer());
    }

    #[test]
    fn test_is_customer() {
        let claims = create_test_claims();
        assert!(claims.is_customer());
        assert!(!claims.is_seller());
    }

    #[test]
    fn test_is_not_expired() {
        let claims = create_test_claims();
        assert!(!claims.is_expired());
    }

    #[test]
    fn test_is_expired() {
        let mut claims = create_test_claims();
        claims.exp = chrono::Utc::now().timestamp() - 1;
        assert!(claims.is_expired());
    }

    #[test]
    fn test_remaining_validity() {
        let claims = create_test_claims();
        let remaining = claims.remaining_validity();
        assert!(remaining > 0 && remaining <= 900);
    }
}
