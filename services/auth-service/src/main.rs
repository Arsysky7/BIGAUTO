use axum::Router;
use dotenvy::dotenv;
use std::net::SocketAddr;
use tokio::signal;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod domain;
mod error;
mod handlers;
mod middleware;
mod models;
mod routes;
mod scheduler;
mod utils;

use config::{AppConfig, AppState};
use error::AppResult;

#[tokio::main]
async fn main() -> AppResult<()> {
    // Load environment variables from .env file
    match dotenv() {
        Ok(path) => println!("✅ Loaded .env from: {:?}", path),
        Err(e) => println!("⚠️  Could not load .env file: {} (will use system env)", e),
    }

    // Initialize tracing subscriber for logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "auth_service=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("🚀 Starting Big Auto - Auth Service");

    // Load configuration from environment
    let config = AppConfig::from_env()
        .map_err(|e| error::AppError::InternalError(format!("Failed to load config: {}", e)))?;

    tracing::info!("✅ Configuration loaded successfully");
    tracing::info!("📦 Database URL: {}", mask_connection_string(&config.database_url));
    tracing::info!("📦 Redis URL: {}", config.redis_url);
    tracing::info!("🔧 Environment: {}", config.environment);

    // Security check: Warn jika production tapi masih pakai default JWT secret
    if config.is_production() && config.jwt_secret.contains("change-this") {
        tracing::warn!("⚠️  SECURITY WARNING: Using default JWT secret in production!");
    }

    // Initialize application state (database pool + redis connection)
    tracing::info!("🔌 Connecting to database and Redis...");
    let state = AppState::new()
        .await
        .map_err(|e| error::AppError::InternalError(format!("Failed to initialize app state: {}", e)))?;

    tracing::info!("✅ Database pool created");
    tracing::info!("✅ Redis connection established");

    // Test database and redis connectivity
    let health = state.health_check().await;
    tracing::info!("💚 Health Check: {:?}", health);

    if health.overall != "healthy" {
        tracing::warn!("⚠️ Some services are not fully healthy: {:?}", health);
    }

    // Start background cleanup scheduler (disabled in Railway)
    if std::env::var("DISABLE_SCHEDULER").unwrap_or_default() != "true" {
        tracing::info!("🔄 Starting background cleanup scheduler...");
        let cleanup_scheduler = scheduler::CleanupScheduler::new(state.clone());
        cleanup_scheduler.start();
        tracing::info!("✅ Background cleanup scheduler started");
    } else {
        tracing::warn!("⚠️  Background cleanup scheduler disabled");
    }

    // Create application router with all routes
    let app = create_app(state);

    // Create server address dari config
    let addr = format!("{}:{}", config.server_host, config.server_port);
    let addr = addr.parse::<SocketAddr>().unwrap_or_else(|_| {
        SocketAddr::from(([0, 0, 0, 0], config.server_port))
    });

    tracing::info!("🎧 Server listening on {}", addr);
    tracing::info!("📚 Swagger UI available at http://localhost:{}/swagger-ui", config.server_port);
    tracing::info!("📖 Health check available at http://localhost:{}/health", config.server_port);

    // Start server with graceful shutdown
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::info!("✅ Auth Service is ready to accept requests!");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    Ok(())
}

/// Create application 
fn create_app(state: AppState) -> Router {
    routes::create_router(state)
        .layer(TraceLayer::new_for_http())
}

/// Graceful shutdown signal handler
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("🛑 Received Ctrl+C, shutting down gracefully...");
        },
        _ = terminate => {
            tracing::info!("🛑 Received terminate signal, shutting down gracefully...");
        },
    }
}

/// Mask sensitive parts of connection string for logging
fn mask_connection_string(url: &str) -> String {
    if let Some(at_pos) = url.find('@') {
        if let Some(proto_end) = url.find("://") {
            let proto = &url[..proto_end + 3];
            let host_part = &url[at_pos..];
            return format!("{}***:***{}", proto, host_part);
        }
    }
    "***".to_string()
}
