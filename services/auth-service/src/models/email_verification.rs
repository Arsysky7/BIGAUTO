use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, FromRow};

// Represent email verification token dari database
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct EmailVerification {
    pub id: i32,
    pub user_id: i32,
    pub token: String,
    pub email: String,
    pub is_used: Option<bool>,
    pub expires_at: DateTime<Utc>,
    pub verified_at: Option<DateTime<Utc>>,
    pub sent_count: Option<i32>,
    pub last_sent_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

// Data untuk create verification token baru
#[derive(Debug)]
pub struct NewEmailVerification {
    pub user_id: i32,
    pub token: String,
    pub email: String,
    pub expires_at: DateTime<Utc>,
}

impl EmailVerification {
    // Create verification token baru
    pub async fn create(
        pool: &PgPool,
        data: NewEmailVerification,
    ) -> Result<Self, sqlx::Error> {
        let result = sqlx::query("INSERT INTO email_verifications (user_id, token, email, expires_at) VALUES ($1, $2, $3, $4) RETURNING id, user_id, token, email, is_used, expires_at, verified_at, sent_count, last_sent_at, created_at")
            .bind(data.user_id)
            .bind(data.token)
            .bind(data.email)
            .bind(data.expires_at)
            .fetch_one(pool)
            .await?;

        Ok(EmailVerification::from_row(&result)?)
    }

    // Cari verification token
    pub async fn find_by_token(pool: &PgPool, token: &str) -> Result<Option<Self>, sqlx::Error> {
        let result = sqlx::query("SELECT id, user_id, token, email, is_used, expires_at, verified_at, sent_count, last_sent_at, created_at FROM email_verifications WHERE token = $1")
            .bind(token)
            .fetch_optional(pool)
            .await?;

        match result {
            Some(row) => {
                let verification = EmailVerification::from_row(&row)?;
                Ok(Some(verification))
            }
            None => Ok(None)
        }
    }

    // Cari verification token terakhir untuk user
    pub async fn find_latest_by_user(
        pool: &PgPool,
        user_id: i32,
    ) -> Result<Option<Self>, sqlx::Error> {
        let result = sqlx::query("SELECT id, user_id, token, email, is_used, expires_at, verified_at, sent_count, last_sent_at, created_at FROM email_verifications WHERE user_id = $1 ORDER BY created_at DESC LIMIT 1")
            .bind(user_id)
            .fetch_optional(pool)
            .await?;

        match result {
            Some(row) => {
                let verification = EmailVerification::from_row(&row)?;
                Ok(Some(verification))
            }
            None => Ok(None)
        }
    }

    // Mark token sebagai sudah dipakai
    pub async fn mark_as_used(pool: &PgPool, token_id: i32) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE email_verifications SET is_used = true, verified_at = NOW() WHERE id = $1")
            .bind(token_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    // Increment sent count (untuk resend)
    pub async fn increment_sent_count(pool: &PgPool, token_id: i32) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE email_verifications SET sent_count = sent_count + 1, last_sent_at = NOW() WHERE id = $1")
            .bind(token_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    // Cek apakah token valid (belum expired & belum dipakai)
    pub fn is_valid(&self) -> bool {
        !self.is_used.unwrap_or(false) && self.expires_at > Utc::now()
    }

    // Cleanup expired tokens (dipanggil via cron job)
    pub async fn cleanup_expired(pool: &PgPool) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            "DELETE FROM email_verifications WHERE expires_at < NOW() AND is_used = false"
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }
}
