mod config;
mod domain;
mod repositories;
mod handlers;
mod routes;
mod middleware;
mod utils;
mod error;

use config::AppState;
use routes::create_routes;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::{info};
use tracing_subscriber::{
    layer::SubscriberExt,
    util::SubscriberInitExt
};

/// Entry point dari Payment Service Big Auto
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Setup logging dengan environment
    setup_logging();

    // Create application state (includes database connection)
    let app_state = AppState::from_env().await?;

    info!("🚀 Payment Service starting on {}:{}", app_state.config.server_host, app_state.config.server_port);
    info!("💳 Mode: {} | Midtrans API: {}",
        if app_state.config.midtrans_is_production { "Production" } else { "Sandbox" },
        app_state.config.midtrans_api_url
    );

    // Build dan start server dengan graceful shutdown
    start_server(app_state).await
}

/// Inisialisasi structured logging berdasarkan environment
fn setup_logging() {
    let _log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("payment_service=debug,tower_http=debug"))
        )
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();
}


/// Start server dengan graceful shutdown dan proper middleware
async fn start_server(app_state: AppState) -> Result<(), Box<dyn std::error::Error>> {
    // Build application dengan middleware stack
    let app = create_routes(app_state.clone())
        .await
        .layer(TraceLayer::new_for_http());

    // Bind listener ke configured address
    let listener = TcpListener::bind(format!("{}:{}", app_state.config.server_host, app_state.config.server_port))
        .await?;

    info!("🌐 Server running on http://{}:{}", app_state.config.server_host, app_state.config.server_port);
    info!("📚 API Docs: http://{}:{}/docs", app_state.config.server_host, app_state.config.server_port);
    info!("🏥 Health Check: http://{}:{}/health", app_state.config.server_host, app_state.config.server_port);

    // Setup graceful shutdown signal handler
    let shutdown_signal = async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
        info!("🛑 Received shutdown signal");
    };

    // Run server dengan graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await?;

    info!("✅ Payment Service shutdown successfully");
    Ok(())
}