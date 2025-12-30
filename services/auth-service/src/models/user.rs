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
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
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

// Response model dengan guaranteed timestamps untuk API

impl User {
    // Cari user berdasarkan email 
    pub async fn find_by_email(pool: &PgPool, email: &str) -> Result<Option<Self>, sqlx::Error> {
        // Validasi input untuk mencegah SQL injection
        let normalized_email = email.trim().to_lowercase();

        sqlx::query_as!(
            Self,
            r#"
            SELECT id, email, password_hash, name, phone, is_seller, address, city,
                   profile_photo, business_name, email_verified, email_verified_at,
                   last_login_at, login_count, is_active, deactivated_at,
                   otp_request_count, otp_blocked_until, last_otp_request_at,
                   created_at, updated_at
            FROM users
            WHERE email = $1 AND is_active = true
            "#,
            normalized_email
        )
        .fetch_optional(pool)
        .await
    }

    // Ambil data user berdasarkan ID untuk operasi profil dan validasi
    pub async fn find_by_id(pool: &PgPool, user_id: i32) -> Result<Option<Self>, sqlx::Error> {
        // Validasi input untuk security
        if user_id <= 0 {
            return Ok(None);
        }

        sqlx::query_as!(
            Self,
            r#"
            SELECT id, email, password_hash, name, phone, is_seller, address, city,
                   profile_photo, business_name, email_verified, email_verified_at,
                   last_login_at, login_count, is_active, deactivated_at,
                   otp_request_count, otp_blocked_until, last_otp_request_at,
                   created_at, updated_at
            FROM users
            WHERE id = $1 AND is_active = true
            "#,
            user_id
        )
        .fetch_optional(pool)
        .await
    }

    // Buat user baru dengan validasi dan normalisasi data
    pub async fn create(pool: &PgPool, new_user: NewUser) -> Result<Self, sqlx::Error> {
        // Normalisasi input untuk data consistency
        let normalized_email = new_user.email.trim().to_lowercase();
        let normalized_name = new_user.name.trim().to_string();
        let normalized_phone = new_user.phone.trim().to_string();

        sqlx::query_as!(
            Self,
            r#"
            INSERT INTO users (email, password_hash, name, phone, address, city)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, email, password_hash, name, phone, is_seller, address, city,
                   profile_photo, business_name, email_verified, email_verified_at,
                   last_login_at, login_count, is_active, deactivated_at,
                   otp_request_count, otp_blocked_until, last_otp_request_at,
                   created_at, updated_at
            "#,
            normalized_email,
            new_user.password_hash,
            normalized_name,
            normalized_phone,
            new_user.address,
            new_user.city
        )
        .fetch_one(pool)
        .await
    }

    // Tandai email user sebagai terverifikasi
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

    // Perbarui data login user setelah successful login
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

    // ===== USER ROLE HELPER FUNCTIONS  =====

    /// Check if user is customer 
    pub fn is_customer(&self) -> bool {
        // Semua user adalah customer 
        true
    }

    /// Check if user is seller role
    pub fn is_seller_role(&self) -> bool {
        self.is_seller.unwrap_or(false)
    }

    
    /// Get user role untuk JWT claims
    pub fn get_jwt_role(&self) -> String {
        match (self.is_customer(), self.is_seller_role()) {
            (true, true) => "hybrid".to_string(),
            (true, false) => "customer".to_string(),
            (false, true) => "seller".to_string(),
            (false, false) => "customer".to_string(),
        }
    }

}