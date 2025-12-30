// Main entry point untuk booking-service
use axum::{Router, http::StatusCode};
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    trace::TraceLayer,
};
use dotenvy::dotenv;
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod domain;
mod error;
mod handlers;
mod repositories;
mod routes;
mod scheduler;
mod middleware;
mod utils;

use config::{AppConfig, AppState};
use error::{AppError, AppResult};
use routes::create_router;
use std::sync::Arc;

#[tokio::main]
async fn main() -> AppResult<()> {
    // Load environment variables dari .env file
    match dotenv() {
        Ok(path) => println!("✅ Environment loaded dari: {:?}", path),
        Err(e) => println!("⚠️  Tidak bisa load .env: {} (menggunakan system env)", e),
    }

    // Initialize tracing subscriber untuk structured logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "booking_service=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("🚀 Memulai BIG AUTO - Booking Service");

    // Load konfigurasi dari environment
    let config = AppConfig::from_env()
        .map_err(|e| AppError::InternalError(format!("Gagal load konfigurasi: {}", e)))?;

    tracing::info!("✅ Konfigurasi berhasil dimuat");
    tracing::info!("📦 Database URL: {}", mask_connection_string(&config.database_url));
    tracing::info!("🔧 Environment: {}", config.environment);
    tracing::info!("🌐 Server: {}:{}", config.host(), config.port());

    // Security check: Warning jika production tapi masih pakai default values
    if config.environment == "production" && config.jwt_secret.contains("change-this") {
        tracing::warn!("⚠️  WARNING: Masih menggunakan default JWT_SECRET di production!");
    }

    // Buat application state dengan database connection
    let app_state = AppState::new(config).await
        .map_err(|e| AppError::internal(format!("Gagal inisialisasi app state: {}", e)))?;

    // Database connection sudah diuji saat create pool
    tracing::info!("✅ Database connection berhasil");

    // Start background cleanup scheduler
    tracing::info!("🔄 Starting background cleanup scheduler...");
    scheduler::BookingScheduler::new(app_state.clone()).start();
    tracing::info!("✅ Background cleanup scheduler started");

    // Build router dengan middleware
    let app = create_app(app_state.clone()).await;

    // Bind server ke address yang ditentukan
    let addr = SocketAddr::from(([0, 0, 0, 0], app_state.config.port()));

    // Start server dengan graceful shutdown
    tracing::info!("🌐 Server listening on http://{}", addr);
    tracing::info!("📚 API Documentation:");
    tracing::info!("   - Swagger UI: http://{}:{}/swagger-ui", app_state.config.host(), app_state.config.port());
    tracing::info!("   - ReDoc: http://{}:{}/redoc", app_state.config.host(), app_state.config.port());
    tracing::info!("   - OpenAPI JSON: http://{}:{}/api-docs/openapi.json", app_state.config.host(), app_state.config.port());

    let listener = tokio::net::TcpListener::bind(addr).await
        .map_err(|e| AppError::internal(format!("Gagal bind server: {}", e)))?;

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| AppError::internal(format!("Server error: {}", e)))?;

    tracing::info!("✅ Server shutdown gracefully");
    Ok(())
}

// Build aplikasi dengan middleware yang sesuai
async fn create_app(state: AppState) -> Router {
    // Convert state ke Arc untuk konsistensi dengan router
    let arc_state = Arc::new(state);

    // Build router dengan semua middleware
    Router::new()
        .merge(create_router(arc_state.as_ref().clone()))
        .fallback(not_found_handler)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CompressionLayer::new())
                .layer(create_cors_layer())
        )
}

// JWT-Only CORS configuration 
fn create_cors_layer() -> CorsLayer {
    use axum::http::{HeaderValue, Method};
    use std::env;
    use std::time::Duration;

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
        .unwrap_or_else(|_| "86400".to_string())
        .parse::<u64>()
        .unwrap_or(86400);

    tracing::info!("🌐 CORS enabled for origins: {}", allowed_origins);

    // JWT-Only CORS Configuration 
    let mut cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::ACCEPT,
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

// Handler untuk 404 errors
async fn not_found_handler() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "API endpoint tidak ditemukan")
}

// Signal handler untuk graceful shutdown
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to setup Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        let mut signal = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to setup terminate signal handler");
        signal.recv().await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<(), ()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("🛑 Received Ctrl+C, shutting down gracefully...");
        },
        _ = terminate => {
            tracing::info!("🛑 Received terminate signal, shutting down gracefully...");
        },
    }
}

// Helper untuk mask connection string di logs (security)
fn mask_connection_string(conn_str: &str) -> String {
    if conn_str.is_empty() {
        return "Not Set".to_string();
    }

    // Extract password dari connection string dan mask dengan asterisks
    if let Some(at_pos) = conn_str.find('@') {
        let (credentials, _) = conn_str.split_at(at_pos);
        if let Some(colon_pos) = credentials.rfind(':') {
            let (user, _) = credentials.split_at(colon_pos);
            format!("{}:****@{}", user, &conn_str[at_pos + 1..])
        } else {
            format!("****@{}", &conn_str[at_pos + 1..])
        }
    } else {
        "Invalid Format".to_string()
    }
}
