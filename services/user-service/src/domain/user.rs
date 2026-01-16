use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, FromRow};
use utoipa::ToSchema;

// Model user dari database
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct User {
    pub id: i32,
    pub email: String,
    #[serde(skip_serializing)]
    #[allow(dead_code)]
    pub password_hash: String,
    pub name: String,
    pub phone: String,
    pub email_verified: bool,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub login_count: i32,
    pub is_active: bool,
    pub deactivated_at: Option<DateTime<Utc>>,
    pub is_seller: bool,
    pub address: Option<String>,
    pub city: Option<String>,
    pub profile_photo: Option<String>,
    pub business_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Response user profile (tanpa data sensitif)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserProfile {
    pub id: i32,
    pub email: String,
    pub name: String,
    pub phone: String,
    pub email_verified: bool,
    pub is_seller: bool,
    pub address: Option<String>,
    pub city: Option<String>,
    pub profile_photo: Option<String>,
    pub business_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserProfile {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            name: user.name,
            phone: user.phone,
            email_verified: user.email_verified,
            is_seller: user.is_seller,
            address: user.address,
            city: user.city,
            profile_photo: user.profile_photo,
            business_name: user.business_name,
            created_at: user.created_at,
        }
    }
}

// Request update profile
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[schema(example = json!({
    "name": "John Doe Updated",
    "phone": "081234567890",
    "address": "Jl. Sudirman No. 456",
    "city": "Jakarta",
    "business_name": "Auto Rental Jakarta Premium"
}))]
pub struct UpdateProfileRequest {
    #[schema(example = "John Doe")]
    pub name: Option<String>,
    #[schema(example = "081234567890")]
    pub phone: Option<String>,
    #[schema(example = "Jl. Sudirman No. 123, Jakarta Selatan")]
    pub address: Option<String>,
    #[schema(example = "Jakarta")]
    pub city: Option<String>,
    #[schema(example = "Auto Rental Jakarta")]
    pub business_name: Option<String>,
}

// Request upgrade to seller
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[schema(example = json!({
    "business_name": "Auto Rental Jakarta"
}))]
pub struct UpgradeToSellerRequest {
    /// Business name (required, minimal 3 karakter)
    #[schema(example = "Auto Rental Jakarta")]
    pub business_name: String,
}

// Response upload photo
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "url": "https://res.cloudinary.com/drjf5hd0p/image/upload/v1234/profiles/user-1.jpg",
    "thumbnail": "https://res.cloudinary.com/drjf5hd0p/image/upload/c_thumb,w_150/v1234/profiles/user-1.jpg"
}))]
pub struct UploadPhotoResponse {
    /// Full image URL dari Cloudinary
    #[schema(example = "https://res.cloudinary.com/drjf5hd0p/image/upload/v1234/profiles/user-1.jpg")]
    pub url: String,
    /// Thumbnail URL (150px width)
    #[schema(example = "https://res.cloudinary.com/drjf5hd0p/image/upload/c_thumb,w_150/v1234/profiles/user-1.jpg")]
    pub thumbnail: String,
}

impl User {
    /// Cleanup expired email verification tokens (older than 24 hours)
    pub async fn cleanup_expired_verifications(pool: &PgPool) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            "DELETE FROM email_verifications WHERE expires_at < NOW() - INTERVAL '24 hours'"
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Deactivate inactive users (not logged in for 365 days)
    pub async fn deactivate_inactive_users(pool: &PgPool) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE users SET is_active = false, deactivated_at = NOW(), updated_at = NOW() WHERE is_active = true AND (last_login_at IS NULL OR last_login_at < NOW() - INTERVAL '365 days')"
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }
}

