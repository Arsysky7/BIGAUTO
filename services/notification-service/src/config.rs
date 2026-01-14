use sqlx::{
    postgres::PgPoolOptions,
    PgPool
};
use std::env;
use std::time::Duration;
use crate::middleware::rate_limit::RateLimiter;

/// Konfigurasi utama aplikasi yang di-load dari environment variables
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub jwt_secret: String,
    pub server_host: String,
    pub server_port: u16,
    pub environment: String,
    pub resend_api_key: String,
    pub resend_from_email: String,
    pub frontend_url: String,
}

impl AppConfig {
    /// Load semua konfigurasi dari env file dengan validasi
    pub fn from_env() -> Result<Self, String> {
        let database_url = env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL environment variable harus diset")?;

        let jwt_secret = env::var("JWT_SECRET")
            .map_err(|_| "JWT_SECRET environment variable harus diset")?;

        // Validasi JWT secret tidak menggunakan default value di production
        if !cfg!(debug_assertions) && jwt_secret.contains("change-this") {
            return Err("JWT_SECRET masih menggunakan default value! Ganti dengan value yang aman untuk production".to_string());
        }

        let server_host = env::var("NOTIFICATION_SERVICE_HOST")
            .unwrap_or_else(|_| "0.0.0.0".to_string());

        let server_port = env::var("NOTIFICATION_SERVICE_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3007);

        let environment = env::var("RUST_ENV").unwrap_or_else(|_| "development".to_string());

        let resend_api_key = env::var("RESEND_API_KEY")
            .map_err(|_| "RESEND_API_KEY environment variable harus diset untuk email notification")?;

        let resend_from_email = env::var("RESEND_FROM_EMAIL")
            .map_err(|_| "RESEND_FROM_EMAIL environment variable harus diset")?;

        let frontend_url = env::var("FRONTEND_URL")
            .map_err(|_| "FRONTEND_URL environment variable harus diset")?;

        Ok(AppConfig {
            database_url,
            jwt_secret,
            server_host,
            server_port,
            environment,
            resend_api_key,
            resend_from_email,
            frontend_url,
        })
    }

    /// Helper untuk cek apakah running di production
    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }
}

/// Inisialisasi database connection pool
pub async fn init_db_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    tracing::info!("🔌 Initializing Notification Service database connection...");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .min_connections(2)
        .acquire_timeout(Duration::from_secs(20))
        .idle_timeout(Duration::from_secs(600))
        .max_lifetime(Duration::from_secs(1800))
        .test_before_acquire(true)
        .connect(database_url)
        .await?;

    tracing::info!("✅ Notification Service database pool initialized successfully");
    Ok(pool)
}

/// Health check untuk database connection
pub async fn check_db_health(pool: &PgPool) -> bool {
    sqlx::query("SELECT 1")
        .fetch_one(pool)
        .await
        .is_ok()
}

/// State aplikasi yang akan di-share ke semua handlers
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: AppConfig,
    pub rate_limiter: RateLimiter,
    pub http_client: reqwest::Client,
}

/// Implement FromRef untuk bisa extract PgPool dari AppState
impl axum::extract::FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}

/// Implement FromRef untuk bisa extract AppConfig dari AppState
impl axum::extract::FromRef<AppState> for AppConfig {
    fn from_ref(state: &AppState) -> Self {
        state.config.clone()
    }
}

/// Implement FromRef untuk bisa extract RateLimiter dari AppState
impl axum::extract::FromRef<AppState> for RateLimiter {
    fn from_ref(state: &AppState) -> Self {
        state.rate_limiter.clone()
    }
}

impl AppState {
    /// Buat AppState baru dengan semua dependensi
    pub async fn new() -> Result<Self, String> {
        let config = AppConfig::from_env()?;
        let db = init_db_pool(&config.database_url)
            .await
            .map_err(|e| format!("Gagal menginisialisasi database: {}", e))?;

        // Initialize Redis rate limiter
        let redis_url = env::var("REDIS_URL")
            .map_err(|_| "REDIS_URL environment variable harus diset untuk rate limiting")?;

        tracing::info!("🔄 Initializing Redis rate limiter...");
        let rate_limiter = RateLimiter::new(&redis_url)
            .map_err(|e| format!("Gagal menginisialisasi Redis rate limiter: {}", e))?;
        tracing::info!("✅ Redis rate limiter initialized successfully");

        // HTTP client untuk Resend API
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| format!("Gagal menginisialisasi HTTP client: {}", e))?;

        Ok(AppState {
            db,
            config,
            rate_limiter,
            http_client,
        })
    }

    /// Health check untuk dependencies
    pub async fn health_check(&self) -> HealthStatus {
        let db_healthy = check_db_health(&self.db).await;

        HealthStatus {
            database: if db_healthy { "healthy" } else { "unhealthy" }.to_string(),
            overall: if db_healthy {
                "healthy"
            } else {
                "degraded"
            }
            .to_string(),
        }
    }
}

/// Struktur untuk response health check endpoint
#[derive(Debug, serde::Serialize)]
pub struct HealthStatus {
    pub database: String,
    pub overall: String,
}