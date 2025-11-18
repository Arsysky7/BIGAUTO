use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};

// Represent user row dari database
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: i32,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub name: String,
    pub phone: String,
    pub is_seller: Option<bool>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub profile_photo: Option<String>,
    pub business_name: Option<String>,
    pub email_verified: Option<bool>,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub login_count: Option<i32>,
    pub is_active: Option<bool>,
    pub deactivated_at: Option<DateTime<Utc>>,
    pub otp_request_count: Option<i32>,
    pub otp_blocked_until: Option<DateTime<Utc>>,
    pub last_otp_request_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}


// Data untuk create user baru
#[derive(Debug, Deserialize)]
pub struct NewUser {
    pub email: String,
    pub password_hash: String,
    pub name: String,
    pub phone: String,
    pub address: Option<String>,
    pub city: Option<String>,
}

// Data untuk update profile
#[derive(Debug, Deserialize)]
pub struct UpdateUserProfile {
    pub name: Option<String>,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub profile_photo: Option<String>,
}

impl User {
    // Cari user berdasarkan email
    pub async fn find_by_email(pool: &PgPool, email: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            SELECT id, email, password_hash, name, phone, is_seller,
                   address, city, profile_photo, business_name,
                   email_verified, email_verified_at, last_login_at,
                   login_count, is_active, deactivated_at,
                   otp_request_count, otp_blocked_until, last_otp_request_at,
                   created_at, updated_at
            FROM users
            WHERE email = $1 AND is_active = true
            "#
        )
        .bind(email)
        .fetch_optional(pool)
        .await
    }

    // Cari user berdasarkan id
    pub async fn find_by_id(pool: &PgPool, user_id: i32) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            SELECT id, email, password_hash, name, phone, is_seller,
                   address, city, profile_photo, business_name,
                   email_verified, email_verified_at, last_login_at,
                   login_count, is_active, deactivated_at,
                   otp_request_count, otp_blocked_until, last_otp_request_at,
                   created_at, updated_at
            FROM users
            WHERE id = $1 AND is_active = true
            "#
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await
    }

    // Create user baru
    pub async fn create(pool: &PgPool, new_user: NewUser) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (email, password_hash, name, phone, address, city)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, email, password_hash, name, phone, is_seller,
                      address, city, profile_photo, business_name,
                      email_verified, email_verified_at, last_login_at,
                      login_count, is_active, deactivated_at,
                      otp_request_count, otp_blocked_until, last_otp_request_at,
                      created_at, updated_at
            "#
        )
        .bind(new_user.email)
        .bind(new_user.password_hash)
        .bind(new_user.name)
        .bind(new_user.phone)
        .bind(new_user.address)
        .bind(new_user.city)
        .fetch_one(pool)
        .await
    }

    // Verifikasi email user
    pub async fn verify_email(pool: &PgPool, user_id: i32) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE users
            SET email_verified = true,
                email_verified_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            "#
        )
        .bind(user_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    // Update login tracking
    pub async fn update_login_tracking(pool: &PgPool, user_id: i32) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE users
            SET last_login_at = NOW(),
                login_count = login_count + 1,
                updated_at = NOW()
            WHERE id = $1
            "#
        )
        .bind(user_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    // Increment OTP request count
    pub async fn increment_otp_request(pool: &PgPool, user_id: i32) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE users
            SET otp_request_count = otp_request_count + 1,
                last_otp_request_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            "#
        )
        .bind(user_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    // NOTE: reset_otp_request_count() dihapus karena tidak ada di REQUIREMENT.MD
    // OTP rate limiting di-handle oleh Redis, bukan manual reset

    // Block user dari request OTP
    pub async fn block_otp_requests(
        pool: &PgPool,
        user_id: i32,
        block_duration_minutes: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE users
            SET otp_blocked_until = NOW() + ($1 * INTERVAL '1 minute'),
                updated_at = NOW()
            WHERE id = $2
            "#
        )
        .bind(block_duration_minutes)
        .bind(user_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    // Check apakah user sedang diblokir dari request OTP
    // Digunakan untuk endpoint GET /api/auth/otp-status
    pub async fn check_otp_blocked(pool: &PgPool, user_id: i32) -> Result<Option<DateTime<Utc>>, sqlx::Error> {
        let result = sqlx::query(
            r#"
            SELECT otp_blocked_until
            FROM users
            WHERE id = $1
            "#
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await?;

        Ok(result.map(|row| row.get("otp_blocked_until")))
    }

    // Update profile user
    pub async fn update_profile(
        pool: &PgPool,
        user_id: i32,
        update_data: UpdateUserProfile,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE users
            SET name = COALESCE($2, name),
                phone = COALESCE($3, phone),
                address = COALESCE($4, address),
                city = COALESCE($5, city),
                profile_photo = COALESCE($6, profile_photo),
                updated_at = NOW()
            WHERE id = $1
            "#
        )
        .bind(user_id)
        .bind(update_data.name)
        .bind(update_data.phone)
        .bind(update_data.address)
        .bind(update_data.city)
        .bind(update_data.profile_photo)
        .execute(pool)
        .await?;
        Ok(())
    }

    // Upgrade ke seller account
    pub async fn upgrade_to_seller(
        pool: &PgPool,
        user_id: i32,
        business_name: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE users
            SET is_seller = true,
                business_name = $2,
                updated_at = NOW()
            WHERE id = $1
            "#
        )
        .bind(user_id)
        .bind(business_name)
        .execute(pool)
        .await?;
        Ok(())
    }

    // NOTE: deactivate() dihapus karena tidak ada fitur deactivate di REQUIREMENT.MD
    // User hanya bisa Customer atau Seller, tidak ada Admin yang bisa deactivate
}
