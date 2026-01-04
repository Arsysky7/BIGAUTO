// Konfigurasi Chat Service - Enterprise-grade configuration management
use serde::Serialize;
use sqlx::{PgPool, postgres::PgConnectOptions, postgres::PgPoolOptions};
use std::env;
use std::time::Duration;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing;

use crate::middleware::rate_limit::RateLimiter;

// Health check response structure
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct HealthCheckResponse {
    pub service: String,
    pub status: String,
    pub version: String,
    pub database: String,
    pub nats: String,
    pub redis: String,
    pub uptime: String,
}

// Application configuration yang di-load dari environment variables
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub server_host: String,
    pub server_port: u16,
    pub environment: String,
    pub jwt_secret: String,
    pub jwt_access_expiry: i64,
    pub jwt_refresh_expiry: i64,
    pub nats_url: String,
    pub redis_url: String,
    pub cloudinary_cloud_name: String,
    pub cloudinary_api_key: String,
    pub cloudinary_api_secret: String,
    pub auth_service_url: String,
    pub user_service_url: String,
    pub vehicle_service_url: String,
    pub booking_service_url: String,
}

impl AppConfig {
    // Load semua konfigurasi dari environment variables dengan validasi
    pub fn from_env() -> Result<Self, String> {
        let database_url = env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL harus diset di environment")?;

        let jwt_secret = env::var("JWT_SECRET")
            .map_err(|_| "JWT_SECRET harus diset di environment")?;

        // Validasi JWT secret tidak menggunakan default value di production
        if !cfg!(debug_assertions) && jwt_secret.contains("change-this") {
            return Err("JWT_SECRET masih menggunakan default value! Ganti dengan value yang aman untuk production".to_string());
        }

        let server_host = env::var("CHAT_SERVICE_HOST")
            .unwrap_or_else(|_| "0.0.0.0".to_string());

        let server_port = env::var("CHAT_SERVICE_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .expect("CHAT_SERVICE_PORT harus diset di environment");

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

        let nats_url = env::var("NATS_URL")
            .expect("NATS_URL harus diset di environment");

        let redis_url = env::var("REDIS_URL")
            .expect("REDIS_URL harus diset di environment");

        let cloudinary_cloud_name = env::var("CLOUDINARY_CLOUD_NAME")
            .expect("CLOUDINARY_CLOUD_NAME harus diset di environment");

        let cloudinary_api_key = env::var("CLOUDINARY_API_KEY")
            .expect("CLOUDINARY_API_KEY harus diset di environment");

        let cloudinary_api_secret = env::var("CLOUDINARY_API_SECRET")
            .expect("CLOUDINARY_API_SECRET harus diset di environment");

        let auth_service_url = env::var("AUTH_SERVICE_URL")
            .expect("AUTH_SERVICE_URL harus diset di environment");

        let user_service_url = env::var("USER_SERVICE_URL")
            .expect("USER_SERVICE_URL harus diset di environment");

        let vehicle_service_url = env::var("VEHICLE_SERVICE_URL")
            .expect("VEHICLE_SERVICE_URL harus diset di environment");

        let booking_service_url = env::var("BOOKING_SERVICE_URL")
            .expect("BOOKING_SERVICE_URL harus diset di environment");

        Ok(AppConfig {
            database_url,
            server_host,
            server_port,
            environment,
            jwt_secret,
            jwt_access_expiry,
            jwt_refresh_expiry,
            nats_url,
            redis_url,
            cloudinary_cloud_name,
            cloudinary_api_key,
            cloudinary_api_secret,
            auth_service_url,
            user_service_url,
            vehicle_service_url,
            booking_service_url,
        })
    }

    // Helper cek production mode
    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }

    // Field aliases untuk kompatibilitas dengan services lainnya
    pub fn host(&self) -> &str {
        &self.server_host
    }

    pub fn port(&self) -> u16 {
        self.server_port
    }

    /// Kemudahan untuk pattern yang konsisten dengan services lainnya
    pub fn new() -> Self {
        Self::from_env().expect("Failed to load configuration from environment")
    }
}

// Inisialisasi database connection pool dengan optimal configuration
pub async fn init_db_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    tracing::info!("🔌 Initializing Chat Service database connection...");

    // Parse connection options dan disable prepared statements untuk Supabase
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

    tracing::info!("✅ Chat Service database pool initialized successfully");
    Ok(pool)
}

// Health check database connection
pub async fn check_db_health(pool: &PgPool) -> bool {
    sqlx::query("SELECT 1")
        .fetch_optional(pool)
        .await
        .is_ok()
}

// WebSocket Connection Limiter untuk mencegah abuse
#[derive(Debug, Clone)]
pub struct WebSocketConnectionLimiter {
    connections: Arc<RwLock<std::collections::HashMap<i32, i32>>>,
}

impl WebSocketConnectionLimiter {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    pub async fn add_connection(&self, user_id: i32) {
        let mut connections = self.connections.write().await;
        *connections.entry(user_id).or_insert(0) += 1;
        tracing::info!("User {} sekarang memiliki {} WebSocket connections", user_id, connections[&user_id]);
    }

    pub async fn remove_connection(&self, user_id: i32) {
        let mut connections = self.connections.write().await;
        if let Some(count) = connections.get_mut(&user_id) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                connections.remove(&user_id);
            }
        }
        tracing::info!("User {} sekarang memiliki {} WebSocket connections", user_id,
                     connections.get(&user_id).unwrap_or(&0));
    }

    pub async fn get_connection_count(&self, user_id: i32) -> i32 {
        let connections = self.connections.read().await;
        connections.get(&user_id).copied().unwrap_or(0)
    }

    pub async fn can_add_connection(&self, user_id: i32, max_connections: i32) -> bool {
        let current = self.get_connection_count(user_id).await;
        current < max_connections
    }
}

// Application state yang di-share ke semua handlers
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: AppConfig,
    pub http_client: reqwest::Client,
    pub nats_client: Option<async_nats::Client>,
    pub message_repo: crate::repositories::MessageRepository,
    pub conversation_repo: crate::repositories::ConversationRepository,
    pub ws_limiter: WebSocketConnectionLimiter,
    pub rate_limiter: Arc<RateLimiter>,
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

        // Initialize NATS client
        let nats_client = match async_nats::connect(&config.nats_url).await {
            Ok(client) => {
                tracing::info!("✅ Terhubung ke NATS server");
                Some(client)
            }
            Err(e) => {
                tracing::warn!("⚠️  Gagal terhubung ke NATS: {}. Real-time features tidak akan tersedia.", e);
                None
            }
        };

        // Initialize repositories
        let message_repo = crate::repositories::MessageRepository::new(db.clone());
        let conversation_repo = crate::repositories::ConversationRepository::new(db.clone());

        // Initialize WebSocket connection limiter
        let ws_limiter = WebSocketConnectionLimiter::new();

        // Redis untuk rate limiting 
        tracing::info!("🔄 Initializing Redis rate limiter...");
        let rate_limiter = RateLimiter::new(&config.redis_url)
            .unwrap_or_else(|e| {
                tracing::error!("❌ Failed to initialize Redis rate limiter: {}", e);
                panic!("Failed to initialize Redis rate limiter: {}. Redis is MANDATORY", e);
            });
        tracing::info!("✅ Redis rate limiter initialized (MANDATORY)");

        Ok(AppState {
            db,
            config,
            http_client,
            nats_client,
            message_repo,
            conversation_repo,
            ws_limiter,
            rate_limiter: Arc::new(rate_limiter),
        })
    }

    // Inisialisasi application state dari environment
    pub async fn from_env() -> Result<Self, String> {
        let config = AppConfig::from_env()?;
        Self::new(config).await
    }

    // Health check semua dependencies
    pub async fn health_check(&self) -> HealthCheckResponse {
        let db_healthy = check_db_health(&self.db).await;

        let nats_status = match &self.nats_client {
            Some(client) => match client.connection_state() {
                async_nats::connection::State::Connected => "connected".to_string(),
                _ => "disconnected".to_string(),
            },
            None => "not_initialized".to_string(),
        };

        // Check Redis connection
        let redis_status = match self.rate_limiter.check_rate_limit("health_check", "guest", "/health").await {
            Ok(_) => "connected".to_string(),
            Err(_) => "disconnected".to_string(),
        };

        let overall_status = if db_healthy && (nats_status == "connected" || nats_status == "not_initialized") && redis_status == "connected" {
            "healthy"
        } else {
            "degraded"
        };

        HealthCheckResponse {
            service: "chat-service".to_string(),
            status: overall_status.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            database: if db_healthy { "connected".to_string() } else { "disconnected".to_string() },
            nats: nats_status,
            redis: redis_status,
            uptime: chrono::Utc::now().to_rfc3339(),
        }
    }

    // Test database connection
    pub async fn test_database_connection(&self) -> Result<(), String> {
        check_db_health(&self.db)
            .await
            .then_some(())
            .ok_or_else(|| "Database connection failed".to_string())
    }
}