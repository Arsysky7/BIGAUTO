// Main Entry Point untuk Chat Service
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod domain;
mod error;
mod handlers;
mod middleware;
mod repositories;
mod routes;
mod utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Initialize structured logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "chat_service=debug,tower_http=debug,async_nats=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("💬 Starting Big Auto - Chat Service");
    tracing::info!("🔧 Real-time messaging with WebSocket & NATS support");

    tracing::info!("🔌 Initializing application state...");
    let config = config::AppConfig::from_env()
        .map_err(|e| format!("Failed to load configuration: {}", e))?;
    let state = config::AppState::new(config).await
        .map_err(|e| format!("Failed to initialize app state: {}", e))?;
    tracing::info!("✅ Application state initialized");

    // Health check untuk dependencies
    tracing::info!("🔍 Performing health checks...");

    // Database health check
    let db_healthy = sqlx::query_scalar!("SELECT 1")
        .fetch_one(&state.db)
        .await
        .is_ok();

    if db_healthy {
        tracing::info!("✅ Database connection healthy");
    } else {
        tracing::error!("❌ Database connection failed");
        return Err("Database health check failed".into());
    }

    // NATS health check
    if let Some(ref nats_client) = state.nats_client {
        match nats_client.connection_state() {
            async_nats::connection::State::Connected => {
                tracing::info!("✅ NATS connection healthy");
            }
            _ => {
                tracing::warn!("⚠️ NATS connection not ready - real-time features may be limited");
            }
        }
    } else {
        tracing::warn!("⚠️ NATS client not initialized - running without real-time messaging");
    }

    // WebSocket limiter health check
    tracing::info!("🔗 WebSocket connection limiter initialized");
    tracing::info!("📊 Max connections per user: 3");

    tracing::info!("🌍 Environment: {}", state.config.environment);
    if state.config.is_production() {
        tracing::warn!("🚨 Running in PRODUCTION mode - all security features enabled");
    } else {
        tracing::info!("🧪 Running in DEVELOPMENT mode - relaxed validation");
    }

    // Build application dengan semua layers
    let app = routes::create_router(state.clone());

    // Setup server address
    let addr = format!("0.0.0.0:{}", state.config.port());

    tracing::info!("🎯 Chat Service listening on {}", addr);
    tracing::info!("📚 API Documentation:");
    tracing::info!("   - Swagger UI: http://{}/docs", addr);
    tracing::info!("   - ReDoc: http://{}/redoc", addr);
    tracing::info!("   - Health Check: http://{}/api/health", addr);
    tracing::info!("🔌 WebSocket Endpoint: ws://{}/api/ws/chat/:conversation_id", addr);

    tracing::info!("🚀 Chat Service Features:");
    tracing::info!("   ✅ Real-time messaging (WebSocket)");
    tracing::info!("   ✅ File & media upload (Cloudinary)");
    tracing::info!("   ✅ Message search & filtering");
    tracing::info!("   ✅ Typing indicators & read receipts");
    tracing::info!("   ✅ JWT-Only authentication");
    tracing::info!("   ✅ Redis-based rate limiting");
    tracing::info!("   ✅ Security headers");

    // Graceful shutdown setup
    let shutdown_signal = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Expect ctrl-c signal");
        tracing::info!("🛑 Received shutdown signal");
    };

    // Start server dengan graceful shutdown
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("🌐 Server bound to {}", addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await?;

    tracing::info!("👋 Chat Service shutdown complete");

    Ok(())
}