use sqlx::{postgres::PgPoolOptions, PgPool};
use std::env;
use std::time::Duration;

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
            .unwrap_or_else(|_| "0.0.0.0".to_string());

        let server_port = env::var("BOOKING_SERVICE_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3004);

        let environment = env::var("RUST_ENV")
            .unwrap_or_else(|_| "development".to_string());

        let vehicle_service_url = env::var("VEHICLE_SERVICE_URL")
            .unwrap_or_else(|_| "http://localhost:3001".to_string());

        let auth_service_url = env::var("AUTH_SERVICE_URL")
            .unwrap_or_else(|_| "http://localhost:3002".to_string());

        let user_service_url = env::var("USER_SERVICE_URL")
            .unwrap_or_else(|_| "http://localhost:3003".to_string());

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

// Inisialisasi database pool dengan konfigurasi optimal
pub async fn init_db_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    tracing::info!("Connecting to PostgreSQL...");

    let pool = PgPoolOptions::new()
        .max_connections(3)
        .min_connections(0)
        .acquire_timeout(Duration::from_secs(30))
        .idle_timeout(Duration::from_secs(600))
        .max_lifetime(Duration::from_secs(1800))
        .connect(database_url)
        .await?;

    tracing::info!("Database connected");
    Ok(pool)
}

// Health check database connection
pub async fn check_db_health(pool: &PgPool) -> bool {
    // Simple connection test - just try to execute query
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

        Ok(AppState {
            db,
            config,
            http_client,
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
