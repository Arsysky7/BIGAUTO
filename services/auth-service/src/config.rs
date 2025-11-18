use redis::aio::ConnectionManager;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::env;
use std::time::Duration;
use crate::utils::email::EmailConfig;

// Konfigurasi utama aplikasi yang di-load dari environment variables
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub redis_url: String,
    pub jwt_secret: String,
    pub jwt_access_expiry: i64,
    pub jwt_refresh_expiry: i64,
    pub server_port: u16,
    pub environment: String,
    pub email_config: EmailConfig,
}

impl AppConfig {
    // Load semua konfigurasi dari env file dengan validasi
    pub fn from_env() -> Result<Self, String> {
        // Note: dotenv() already called in main.rs before this function
        let database_url = env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL harus diset di environment")?;

        let redis_url = env::var("REDIS_URL")
            .map_err(|_| "REDIS_URL harus diset di environment")?;

        let jwt_secret = env::var("JWT_SECRET")
            .map_err(|_| "JWT_SECRET harus diset di environment")?;

        // validasi JWT secret tidak menggunakan default value di production
        if !cfg!(debug_assertions) && jwt_secret.contains("change-this") {
            return Err("JWT_SECRET masih menggunakan default value! Ganti dengan value yang aman untuk production".to_string());
        }

        let jwt_access_expiry = env::var("JWT_ACCESS_TOKEN_EXPIRY")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(900);

        let jwt_refresh_expiry = env::var("JWT_REFRESH_TOKEN_EXPIRY")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(604800);

        let server_port = env::var("AUTH_SERVICE_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3001);

        let environment = env::var("RUST_ENV").unwrap_or_else(|_| "development".to_string());

        let email_config = EmailConfig::from_env()
            .map_err(|e| format!("Email config error: {}", e))?;

        Ok(AppConfig {
            database_url,
            redis_url,
            jwt_secret,
            jwt_access_expiry,
            jwt_refresh_expiry,
            server_port,
            environment,
            email_config,
        })
    }

    // Helper untuk cek apakah running di production
    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }
}

// Inisialisasi connection pool database dengan konfigurasi optimal
pub async fn init_db_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    tracing::info!("Menghubungkan ke database PostgreSQL...");

    // Add statement_cache_mode=disable to prevent prepared statement conflicts
    let modified_url = if database_url.contains('?') {
        format!("{}&statement_cache_mode=disable", database_url)
    } else {
        format!("{}?statement_cache_mode=disable", database_url)
    };

    tracing::info!("Database URL dengan statement cache disabled: {}", &modified_url.split('@').nth(1).unwrap_or("****"));

    let pool = PgPoolOptions::new()
        .max_connections(2)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(15))
        .idle_timeout(Duration::from_secs(300))
        .max_lifetime(Duration::from_secs(900))
        .test_before_acquire(true)
        .connect(&modified_url)
        .await?;

    tracing::info!("Koneksi database berhasil dibuat");

    Ok(pool)
}

// Inisialisasi Redis connection manager untuk caching dan rate limiting
pub async fn init_redis_manager(redis_url: &str) -> Result<ConnectionManager, redis::RedisError> {
    tracing::info!("Menghubungkan ke Redis...");

    let client = redis::Client::open(redis_url)?;
    let manager = ConnectionManager::new(client).await?;

    tracing::info!("Koneksi Redis berhasil dibuat");

    Ok(manager)
}

// Health check untuk database connection
pub async fn check_db_health(pool: &PgPool) -> bool {
    sqlx::query("SELECT 1")
        .fetch_one(pool)
        .await
        .is_ok()
}

// Health check untuk Redis connection
pub async fn check_redis_health(manager: &mut ConnectionManager) -> bool {
    use redis::AsyncCommands;
    manager.get::<_, Option<String>>("__health_check__")
        .await
        .is_ok()
}

// State aplikasi yang akan di-share ke semua handlers
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub redis: ConnectionManager,
    pub config: AppConfig,
    pub http_client: reqwest::Client,
}

impl AppState {
    // Buat AppState baru dengan semua dependensi
    pub async fn new() -> Result<Self, String> {
        let config = AppConfig::from_env()?;
        let db = init_db_pool(&config.database_url)
            .await
            .map_err(|e| format!("Gagal menginisialisasi database: {}", e))?;
        let redis = init_redis_manager(&config.redis_url)
            .await
            .map_err(|e| format!("Gagal menginisialisasi Redis: {}", e))?;

        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| format!("Gagal menginisialisasi HTTP client: {}", e))?;

        Ok(AppState { db, redis, config, http_client })
    }

    // Health check untuk semua dependencies
    pub async fn health_check(&self) -> HealthStatus {
        let db_healthy = check_db_health(&self.db).await;
        let mut redis_conn = self.redis.clone();
        let redis_healthy = check_redis_health(&mut redis_conn).await;

        HealthStatus {
            database: if db_healthy { "healthy" } else { "unhealthy" }.to_string(),
            redis: if redis_healthy { "healthy" } else { "unhealthy" }.to_string(),
            overall: if db_healthy && redis_healthy {
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
    pub redis: String,
    pub overall: String,
}
