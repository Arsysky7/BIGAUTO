use sqlx::{postgres::PgPoolOptions, postgres::PgConnectOptions, PgPool};
use std::env;
use std::time::Duration;
use crate::middleware::rate_limit::RateLimiter;

// Konfigurasi aplikasi dari environment variables
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub server_host: String,
    pub server_port: u16,
    pub environment: String,
    pub jwt_secret: String,
    pub vehicle_service_url: String,
    pub auth_service_url: String,
    pub user_service_url: String,
}

impl AppConfig {
    // Load konfigurasi dari environment dengan validasi
    pub fn from_env() -> Result<Self, String> {
        let database_url = env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL harus diset")?;

        let jwt_secret = env::var("JWT_SECRET")
            .map_err(|_| "JWT_SECRET harus diset")?;

        if !cfg!(debug_assertions) && jwt_secret.contains("change-this") {
            return Err("JWT_SECRET masih default! Ganti untuk production".to_string());
        }

        let server_host = env::var("BOOKING_SERVICE_HOST")
            .expect("BOOKING_SERVICE_HOST harus diset di environment");

        let server_port = env::var("BOOKING_SERVICE_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .expect("BOOKING_SERVICE_PORT harus diset di environment");

        let environment = env::var("RUST_ENV")
            .expect("RUST_ENV harus diset di environment");

        let vehicle_service_url = env::var("VEHICLE_SERVICE_URL")
            .expect("VEHICLE_SERVICE_URL harus diset di environment");

        let auth_service_url = env::var("AUTH_SERVICE_URL")
            .expect("AUTH_SERVICE_URL harus diset di environment");

        let user_service_url = env::var("USER_SERVICE_URL")
            .expect("USER_SERVICE_URL harus diset di environment");

        Ok(AppConfig {
            database_url,
            server_host,
            server_port,
            environment,
            jwt_secret,
            vehicle_service_url,
            auth_service_url,
            user_service_url,
        })
    }

    // Helper cek production mode
    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }

    // Field aliases untuk kompatibilitas
    pub fn host(&self) -> &str {
        &self.server_host
    }

    pub fn port(&self) -> u16 {
        self.server_port
    }
}

// Inisialisasi database connection pool
pub async fn init_db_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    tracing::info!("🔌 Initializing Booking Service database connection...");

    // Parse connection options dan disable prepared statements 
    use std::str::FromStr;
    let options = PgConnectOptions::from_str(database_url)?
        .statement_cache_capacity(0);       

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .min_connections(3)
        .acquire_timeout(Duration::from_secs(10))
        .idle_timeout(Duration::from_secs(300))
        .max_lifetime(Duration::from_secs(1800))
        .test_before_acquire(true)
        .connect_with(options)  
        .await?;

    tracing::info!("✅ Booking Service database pool initialized successfully for Big Auto platform");
    Ok(pool)
}

// Health check database connection
pub async fn check_db_health(pool: &PgPool) -> bool {
    sqlx::query("SELECT 1")
        .fetch_optional(pool)
        .await
        .is_ok()
}

// Application state yang di-share ke semua handlers
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: AppConfig,
    pub http_client: reqwest::Client,
    pub rate_limiter: RateLimiter,
}

impl axum::extract::FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}

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
    // Inisialisasi application state
    pub async fn new(config: AppConfig) -> Result<Self, String> {
        let db = init_db_pool(&config.database_url)
            .await
            .map_err(|e| format!("Failed to init database: {}", e))?;

        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

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

        Ok(AppState {
            db,
            config,
            http_client,
            rate_limiter,
        })
    }

    // Inisialisasi application state dari environment
    pub async fn from_env() -> Result<Self, String> {
        let config = AppConfig::from_env()?;
        Self::new(config).await
    }

    // Test database connection
    pub async fn test_database_connection(&self) -> Result<(), String> {
        check_db_health(&self.db)
            .await
            .then_some(())
            .ok_or_else(|| "Database connection failed".to_string())
    }

    // Health check semua dependencies
    pub async fn health_check(&self) -> HealthStatus {
        let db_healthy = check_db_health(&self.db).await;

        HealthStatus {
            database: if db_healthy { "healthy" } else { "unhealthy" }.to_string(),
            overall: if db_healthy { "healthy" } else { "degraded" }.to_string(),
        }
    }
}

// Response untuk health check endpoint
#[derive(Debug, serde::Serialize)]
pub struct HealthStatus {
    pub database: String,
    pub overall: String,
}