use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod domain;
mod error;
mod handlers;
mod middleware;
mod repositories;
mod routes;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "vehicle_service=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("🚗 Starting Big Auto - Vehicle Service");

    tracing::info!("🔌 Initializing application state...");
    let state = config::AppState::new().await
        .map_err(|e| format!("Failed to initialize app state: {}", e))?;
    tracing::info!("✅ Application state initialized");

    let health = state.health_check().await;
    if health.overall == "healthy" {
        tracing::info!("✅ Database health check passed");
    } else {
        tracing::warn!("⚠️ Health check: Database {}", health.database);
    }

    let app = routes::create_router(state.clone())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http());

    let addr = format!("{}:{}", state.config.server_host, state.config.server_port);
    tracing::info!("🎯 Vehicle Service listening on {}", addr);
    tracing::info!("📚 API Documentation:");
    tracing::info!("   - Swagger UI: http://{}/swagger-ui", addr);
    tracing::info!("   - ReDoc: http://{}/redoc", addr);
    tracing::info!("🌍 Environment: {}", state.config.environment);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
