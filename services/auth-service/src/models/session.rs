use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

// Represent user session dari database
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserSession {
    pub id: i32,
    pub user_id: i32,
    pub refresh_token: String,
    pub access_token_jti: Option<String>,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    pub device_name: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub last_activity: Option<DateTime<Utc>>,
    pub is_active: Option<bool>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Data untuk create session baru
#[derive(Debug)]
pub struct NewUserSession {
    pub user_id: i32,
    pub refresh_token: String,
    pub access_token_jti: Option<String>,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    pub device_name: Option<String>,
    pub expires_at: DateTime<Utc>,
}

impl UserSession {
    // Create session baru
    pub async fn create(pool: &PgPool, data: NewUserSession) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, UserSession>(
            r#"
            INSERT INTO user_sessions
                (user_id, refresh_token, access_token_jti, user_agent, ip_address, device_name, expires_at)
            VALUES ($1, $2, $3, $4, $5::inet, $6, $7)
            RETURNING id, user_id, refresh_token, access_token_jti,
                      user_agent, ip_address::text, device_name,
                      expires_at, last_activity, is_active,
                      created_at, updated_at
            "#
        )
        .bind(data.user_id)
        .bind(data.refresh_token)
        .bind(data.access_token_jti)
        .bind(data.user_agent)
        .bind(data.ip_address)
        .bind(data.device_name)
        .bind(data.expires_at)
        .fetch_one(pool)
        .await
    }

    // Cari session by refresh token
    pub async fn find_by_refresh_token(
        pool: &PgPool,
        refresh_token: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, UserSession>(
            r#"
            SELECT id, user_id, refresh_token, access_token_jti,
                   user_agent, ip_address::text, device_name,
                   expires_at, last_activity, is_active,
                   created_at, updated_at
            FROM user_sessions
            WHERE refresh_token = $1 AND is_active = true
            "#
        )
        .bind(refresh_token)
        .fetch_optional(pool)
        .await
    }

    // Cari session by ID
    pub async fn find_by_id(pool: &PgPool, session_id: i32) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, UserSession>(
            r#"
            SELECT id, user_id, refresh_token, access_token_jti,
                   user_agent, ip_address::text, device_name,
                   expires_at, last_activity, is_active,
                   created_at, updated_at
            FROM user_sessions
            WHERE id = $1
            "#
        )
        .bind(session_id)
        .fetch_optional(pool)
        .await
    }

    // Cari semua active sessions untuk user
    pub async fn find_active_by_user(
        pool: &PgPool,
        user_id: i32,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, UserSession>(
            r#"
            SELECT id, user_id, refresh_token, access_token_jti,
                   user_agent, ip_address::text, device_name,
                   expires_at, last_activity, is_active,
                   created_at, updated_at
            FROM user_sessions
            WHERE user_id = $1
              AND is_active = true
              AND expires_at > NOW()
            ORDER BY last_activity DESC
            "#
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
    }

    // Update last activity
    pub async fn update_activity(pool: &PgPool, session_id: i32) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE user_sessions
            SET last_activity = NOW(),
                updated_at = NOW()
            WHERE id = $1
            "#
        )
        .bind(session_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    // Update access token JTI untuk tracking dan revocation
    pub async fn update_access_token_jti(
        pool: &PgPool,
        session_id: i32,
        jti: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE user_sessions
            SET access_token_jti = $2,
                updated_at = NOW()
            WHERE id = $1
            "#
        )
        .bind(session_id)
        .bind(jti)
        .execute(pool)
        .await?;
        Ok(())
    }

    // Invalidate session (logout)
    pub async fn invalidate(pool: &PgPool, session_id: i32) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE user_sessions
            SET is_active = false,
                updated_at = NOW()
            WHERE id = $1
            "#
        )
        .bind(session_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    // Invalidate session by refresh token
    pub async fn invalidate_by_token(
        pool: &PgPool,
        refresh_token: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE user_sessions
            SET is_active = false,
                updated_at = NOW()
            WHERE refresh_token = $1
            "#
        )
        .bind(refresh_token)
        .execute(pool)
        .await?;
        Ok(())
    }

    // Invalidate semua sessions untuk user (logout dari semua device)
    pub async fn invalidate_all_by_user(pool: &PgPool, user_id: i32) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE user_sessions
            SET is_active = false,
                updated_at = NOW()
            WHERE user_id = $1 AND is_active = true
            "#
        )
        .bind(user_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    // Cek apakah session valid
    pub fn is_valid(&self) -> bool {
        self.is_active.unwrap_or(false) && self.expires_at > Utc::now()
    }

    // Cleanup expired sessions (dipanggil via cron job)
    pub async fn cleanup_expired(pool: &PgPool) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            "DELETE FROM user_sessions WHERE expires_at < NOW() - INTERVAL '7 days'"
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }

    // Cleanup inactive sessions (lebih dari 30 hari tidak aktif)
    pub async fn cleanup_inactive(pool: &PgPool) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            "DELETE FROM user_sessions WHERE last_activity < NOW() - INTERVAL '30 days'"
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }
}
