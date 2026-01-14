use dotenvy::dotenv;
use serde::Deserialize;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::time::Duration;
use crate::middleware::rate_limit::RateLimiter;

// Konfigurasi aplikasi dari environment variables
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub database_url: String,
    pub jwt_secret: String,
    pub server_host: String,
    pub server_port: u16,
    pub environment: String,
}

// Status untuk health check
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct HealthStatus {
    pub database: String,
    pub overall: String,
}

// Application state dengan dependency injection
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: AppConfig,
    pub rate_limiter: RateLimiter,
}

impl AppConfig {
    // Load konfigurasi dari environment variables
    pub fn from_env() -> Result<Self, String> {
        dotenv().ok();

        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL environment variable harus diset".to_string())?;

        let jwt_secret = std::env::var("JWT_SECRET")
            .map_err(|_| "JWT_SECRET environment variable harus diset".to_string())?;

        let server_host = std::env::var("FINANCIAL_SERVICE_HOST")
            .unwrap_or_else(|_| "0.0.0.0".to_string());

        let server_port = std::env::var("FINANCIAL_SERVICE_PORT")
            .unwrap_or_else(|_| "3008".to_string())
            .parse::<u16>()
            .map_err(|_| "FINANCIAL_SERVICE_PORT harus berupa angka".to_string())?;

        let environment = std::env::var("RUST_ENV")
            .unwrap_or_else(|_| "development".to_string());

        Ok(Self {
            database_url,
            jwt_secret,
            server_host,
            server_port,
            environment,
        })
    }

    // Cek apakah environment production
    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }
}

// PgPool dari AppState
impl axum::extract::FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}

// RateLimiter dari AppState
impl axum::extract::FromRef<AppState> for crate::middleware::rate_limit::RateLimiter {
    fn from_ref(state: &AppState) -> Self {
        state.rate_limiter.clone()
    }
}

impl AppState {
    // Buat AppState baru dengan database connection pool dan rate limiter
    pub async fn new() -> Result<Self, String> {
        let config = AppConfig::from_env()?;

        // Security check untuk production
        if config.is_production() && config.jwt_secret.contains("change-this") {
            return Err("SECURITY WARNING: JWT_SECRET masih menggunakan default value di production".to_string());
        }

        let db = init_db_pool(&config.database_url).await?;

        // Inisialisasi rate limiter dengan Redis dari environment
        let redis_url = std::env::var("REDIS_URL")
            .map_err(|_| "REDIS_URL environment variable harus diset".to_string())?;
        let rate_limiter = RateLimiter::new(&redis_url)
            .map_err(|e| format!("Gagal connect ke Redis: {}", e))?;

        Ok(Self { db, config, rate_limiter })
    }

    // Health check untuk database
    pub async fn health_check(&self) -> HealthStatus {
        let db_healthy = check_db_health(&self.db).await;

        HealthStatus {
            database: if db_healthy { "healthy" } else { "unhealthy" }.to_string(),
            overall: if db_healthy { "healthy" } else { "degraded" }.to_string(),
        }
    }
}

// Inisialisasi database connection pool dengan optimal settings
async fn init_db_pool(database_url: &str) -> Result<PgPool, String> {
    PgPoolOptions::new()
        .max_connections(15)
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(600))
        .max_lifetime(Duration::from_secs(1800))
        .connect(database_url)
        .await
        .map_err(|e| format!("Gagal connect ke database: {}", e))
}

// Cek kesehatan database dengan simple query
pub async fn check_db_health(pool: &PgPool) -> bool {
    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(pool)
        .await
        .is_ok()
}