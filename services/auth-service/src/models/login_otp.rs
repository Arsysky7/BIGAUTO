use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row, FromRow};

// Represent OTP login dari database
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct LoginOtp {
    pub id: i32,
    pub user_id: i32,
    pub otp_code: String,
    pub otp_hash: String,
    pub expires_at: DateTime<Utc>,
    pub is_used: Option<bool>,
    pub used_at: Option<DateTime<Utc>>,
    pub attempt_count: Option<i32>,
    pub blocked_until: Option<DateTime<Utc>>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
}


// Data untuk create OTP baru
#[derive(Debug)]
pub struct NewLoginOtp {
    pub user_id: i32,
    pub otp_code: String,
    pub otp_hash: String,
    pub expires_at: DateTime<Utc>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

impl LoginOtp {
    // Create OTP baru
    pub async fn create(pool: &PgPool, data: NewLoginOtp) -> Result<Self, sqlx::Error> {
        let result = sqlx::query("INSERT INTO login_otps (user_id, otp_code, otp_hash, expires_at, ip_address, user_agent) VALUES ($1, $2, $3, $4, $5::inet, $6) RETURNING id, user_id, otp_code, otp_hash, expires_at, is_used, used_at, attempt_count, blocked_until, ip_address::text, user_agent, created_at")
        .bind(data.user_id)
        .bind(data.otp_code)
        .bind(data.otp_hash)
        .bind(data.expires_at)
        .bind(data.ip_address)
        .bind(data.user_agent)
        .fetch_one(pool)
        .await?;

        Ok(LoginOtp::from_row(&result)?)
    }

    // Cari OTP terakhir untuk user (yang belum expired & belum dipakai)
    pub async fn find_latest_valid_by_user(
        pool: &PgPool,
        user_id: i32,
    ) -> Result<Option<Self>, sqlx::Error> {
        let result = sqlx::query("SELECT id, user_id, otp_code, otp_hash, expires_at, is_used, used_at, attempt_count, blocked_until, ip_address::text, user_agent, created_at FROM login_otps WHERE user_id = $1 AND is_used = false AND expires_at > NOW() ORDER BY created_at DESC LIMIT 1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?;

        match result {
            Some(row) => {
                let otp = LoginOtp::from_row(&row)?;
                Ok(Some(otp))
            }
            None => Ok(None)
        }
    }

    // Increment attempt count
    pub async fn increment_attempt(pool: &PgPool, otp_id: i32) -> Result<i32, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE login_otps SET attempt_count = attempt_count + 1 WHERE id = $1 RETURNING attempt_count"
        )
        .bind(otp_id)
        .fetch_one(pool)
        .await?;

        Ok(result.get::<i32, _>("attempt_count"))
    }

    // Block OTP setelah max attempts
    pub async fn block_otp(
        pool: &PgPool,
        otp_id: i32,
        block_duration_minutes: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE login_otps SET blocked_until = NOW() + ($2 * INTERVAL '1 minute') WHERE id = $1")
        .bind(otp_id)
        .bind(block_duration_minutes)
        .execute(pool)
        .await?;
        Ok(())
    }

    // Mark OTP sebagai sudah dipakai
    pub async fn mark_as_used(pool: &PgPool, otp_id: i32) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE login_otps SET is_used = true, used_at = NOW() WHERE id = $1")
        .bind(otp_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    // Cek apakah OTP valid
    pub fn is_valid(&self) -> bool {
        !self.is_used.unwrap_or(false)
            && self.expires_at > Utc::now()
            && self.blocked_until.map_or(true, |blocked| blocked < Utc::now())
    }

    // Cek apakah OTP sedang diblock
    pub fn is_blocked(&self) -> bool {
        self.blocked_until.map_or(false, |blocked| blocked > Utc::now())
    }

    // Invalidate semua OTP lama untuk user
    pub async fn invalidate_old_otps(pool: &PgPool, user_id: i32) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE login_otps SET is_used = true WHERE user_id = $1 AND is_used = false")
        .bind(user_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    // Cleanup expired OTPs (dipanggil via cron job)
    pub async fn cleanup_expired(pool: &PgPool) -> Result<u64, sqlx::Error> {
        let result = sqlx::query("DELETE FROM login_otps WHERE expires_at < NOW() - INTERVAL '24 hours'")
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }
}
