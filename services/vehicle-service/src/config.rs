use sqlx::{postgres::PgPoolOptions, PgPool};
use std::env;
use std::time::Duration;
use crate::middleware::rate_limit::RateLimiter;

// Konfigurasi utama aplikasi yang di-load dari environment variables
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub server_host: String,
    pub server_port: u16,
    pub environment: String,
}

impl AppConfig {
    // Load semua konfigurasi dari env file dengan validasi
    pub fn from_env() -> Result<Self, String> {
        let database_url = env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL harus diset di environment")?;

        // Validasi JWT_SECRET ada di environment (untuk auth middleware)
        let jwt_secret = env::var("JWT_SECRET")
            .map_err(|_| "JWT_SECRET harus diset di environment")?;

        // Validasi JWT secret tidak menggunakan default value di production
        if !cfg!(debug_assertions) && jwt_secret.contains("change-this") {
            return Err("JWT_SECRET masih menggunakan default value! Ganti dengan value yang aman untuk production".to_string());
        }

        let server_host = env::var("VEHICLE_SERVICE_HOST")
            .unwrap_or_else(|_| "0.0.0.0".to_string());

        let server_port = env::var("VEHICLE_SERVICE_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3003);

        let environment = env::var("RUST_ENV").unwrap_or_else(|_| "development".to_string());

        Ok(AppConfig {
            database_url,
            server_host,
            server_port,
            environment,
        })
    }

    // Helper untuk cek apakah running di production
    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }

    // Helper untuk strict validation di production
    pub fn strict_validation(&self) -> bool {
        self.is_production()
    }
}

// Inisialisasi connection pool database dengan konfigurasi optimal
pub async fn init_db_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    tracing::info!("Menghubungkan ke database PostgreSQL...");

    let pool = PgPoolOptions::new()
        .max_connections(3)
        .min_connections(0)
        .acquire_timeout(Duration::from_secs(30))
        .idle_timeout(Duration::from_secs(600))
        .max_lifetime(Duration::from_secs(1800))
        .connect(database_url)
        .await?;

    tracing::info!("Koneksi database berhasil dibuat");

    Ok(pool)
}

// Health check untuk database connection
pub async fn check_db_health(pool: &PgPool) -> bool {
    sqlx::query("SELECT 1")
        .fetch_one(pool)
        .await
        .is_ok()
}

// State aplikasi yang akan di-share ke semua handlers
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: AppConfig,
    pub rate_limiter: RateLimiter,
}

// Implement FromRef untuk bisa extract PgPool dari AppState
impl axum::extract::FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}

// Implement FromRef untuk bisa extract AppConfig dari AppState
impl axum::extract::FromRef<AppState> for AppConfig {
    fn from_ref(state: &AppState) -> Self {
        state.config.clone()
    }
}

// Implement FromRef untuk bisa extract RateLimiter dari AppState
impl axum::extract::FromRef<AppState> for RateLimiter {
    fn from_ref(state: &AppState) -> Self {
        state.rate_limiter.clone()
    }
}

impl AppState {
    // Buat AppState baru dengan semua dependensi
    pub async fn new() -> Result<Self, String> {
        let config = AppConfig::from_env()?;
        let db = init_db_pool(&config.database_url)
            .await
            .map_err(|e| format!("Gagal menginisialisasi database: {}", e))?;

        // Initialize Redis rate limiter 
        let redis_url = env::var("REDIS_URL")
            .unwrap_or_else(|_| {
                tracing::error!("❌ REDIS_URL environment variable tidak diset");
                panic!("REDIS_URL environment variable is REQUIRED for rate limiting");
            });

        tracing::info!("🔄 Initializing Redis rate limiter...");
        let rate_limiter = RateLimiter::new(&redis_url)
            .unwrap_or_else(|e| {
                tracing::error!("❌ Failed to initialize Redis rate limiter: {}", e);
                panic!("Failed to initialize Redis rate limiter: {}. Redis is MANDATORY", e);
            });
        tracing::info!("✅ Redis rate limiter initialized successfully (MANDATORY)");

        Ok(AppState { db, config, rate_limiter })
    }

    // Health check untuk dependencies
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

// Struktur untuk response health check endpoint
#[derive(Debug, serde::Serialize)]
pub struct HealthStatus {
    pub database: String,
    pub overall: String,
}