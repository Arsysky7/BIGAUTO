// Payment Service Configuration
use sqlx::{postgres::PgPoolOptions, postgres::PgConnectOptions, PgPool};
use std::env;
use std::time::Duration;
use std::str::FromStr;
use crate::repositories::payment_repo::PaymentRepository;
use crate::middleware::rate_limit::RateLimiter;

// Konfigurasi aplikasi dari environment variables
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub server_host: String,
    pub server_port: u16,
    pub environment: String,
    pub jwt_secret: String,
    pub jwt_access_expiry: i64,
    pub jwt_refresh_expiry: i64,
    pub midtrans_server_key: String,
    pub midtrans_client_key: String,
    pub midtrans_is_production: bool,
    pub midtrans_api_url: String,
    pub booking_service_url: String,
    pub user_service_url: String,
    pub app_version: String,
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

        let server_host = env::var("PAYMENT_SERVICE_HOST")
            .expect("PAYMENT_SERVICE_HOST harus diset di environment");

        let server_port = env::var("PAYMENT_SERVICE_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .expect("PAYMENT_SERVICE_PORT harus diset di environment");

        let environment = env::var("RUST_ENV")
            .expect("RUST_ENV harus diset di environment");

        let jwt_access_expiry = env::var("JWT_ACCESS_TOKEN_EXPIRY")
            .ok()
            .and_then(|s| s.parse().ok())
            .expect("JWT_ACCESS_TOKEN_EXPIRY harus diset di environment");

        let jwt_refresh_expiry = env::var("JWT_REFRESH_TOKEN_EXPIRY")
            .ok()
            .and_then(|s| s.parse().ok())
            .expect("JWT_REFRESH_TOKEN_EXPIRY harus diset di environment");

        let midtrans_server_key = env::var("MIDTRANS_SERVER_KEY")
            .expect("MIDTRANS_SERVER_KEY harus diset di environment");

        let midtrans_client_key = env::var("MIDTRANS_CLIENT_KEY")
            .expect("MIDTRANS_CLIENT_KEY harus diset di environment");

        let midtrans_is_production = env::var("MIDTRANS_IS_PRODUCTION")
            .ok()
            .and_then(|s| s.parse().ok())
            .expect("MIDTRANS_IS_PRODUCTION harus diset di environment");

        let midtrans_api_url = env::var("MIDTRANS_API_URL")
            .expect("MIDTRANS_API_URL harus diset di environment");

        let booking_service_url = env::var("BOOKING_SERVICE_URL")
            .expect("BOOKING_SERVICE_URL harus diset di environment");

        let user_service_url = env::var("USER_SERVICE_URL")
            .expect("USER_SERVICE_URL harus diset di environment");

        let app_version = env::var("APP_VERSION")
            .unwrap_or_else(|_| "1.0.0".to_string());

        Ok(AppConfig {
            database_url,
            server_host,
            server_port,
            environment,
            jwt_secret,
            jwt_access_expiry,
            jwt_refresh_expiry,
            midtrans_server_key,
            midtrans_client_key,
            midtrans_is_production,
            midtrans_api_url,
            booking_service_url,
            user_service_url,
            app_version,
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

    /// Kemudahan untuk pattern yang konsisten dengan services lain
    pub fn new() -> Self {
        Self::from_env().expect("Failed to load configuration from environment")
    }
}

// Inisialisasi database connection pool
pub async fn init_db_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    tracing::info!("🔌 Initializing Payment Service database connection...");

    // Parse connection options dan disable prepared statements 
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

    tracing::info!("✅ Payment Service database pool initialized successfully for Big Auto platform");
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
    pub payment_repository: PaymentRepository,
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

        let payment_repository = PaymentRepository::new(db.clone());

        // Redis MANDATORY untuk rate limiting 
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
        tracing::info!("✅ Redis rate limiter initialized (MANDATORY)");

        Ok(AppState {
            db,
            config,
            http_client,
            payment_repository,
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

