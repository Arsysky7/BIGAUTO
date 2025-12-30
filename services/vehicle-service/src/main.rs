use std::time::Duration;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod domain;
mod error;
mod handlers;
mod middleware;
mod repositories;
mod routes;
mod scheduler;
mod utils;

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

    // Start background cleanup scheduler
    tracing::info!("🔄 Starting background cleanup scheduler...");
    scheduler::VehicleScheduler::new(state.clone()).start();
    tracing::info!("✅ Background cleanup scheduler started");

    let app = routes::create_router(state.clone())
        .layer(create_cors_layer())
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

// Membuat CORS layer yang aman berdasarkan environment variables
fn create_cors_layer() -> CorsLayer {
    use axum::http::{HeaderValue, Method};
    use std::env;

    // Allowed origins dari environment variable
    let allowed_origins = env::var("FRONTEND_URL")
        .expect("FRONTEND_URL environment variable HARUS diisi di .env file");

    // Parse origins yang diperbolehkan
    let origins: Vec<HeaderValue> = allowed_origins
        .split(',')
        .filter_map(|origin| origin.trim().parse::<HeaderValue>().ok())
        .collect();

    // CORS max age dari environment
    let max_age_seconds = env::var("CORS_MAX_AGE_SECONDS")
        .expect("CORS_MAX_AGE_SECONDS environment variable HARUS diisi di .env file")
        .parse::<u64>()
        .expect("CORS_MAX_AGE_SECONDS harus berupa angka (detik)");

    // Build CORS layer dengan origins yang dinamis
    let mut cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
        ])
        .allow_credentials(false)  
        .max_age(Duration::from_secs(max_age_seconds));

    // Tambahkan semua allowed origins
    for origin in origins {
        cors = cors.allow_origin(origin);
    }

    cors
}
