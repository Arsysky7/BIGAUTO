// Main entry point untuk booking-service
// Mengelola rental bookings, test drive bookings, dan sale orders
use axum::{Router, http::{StatusCode, header::HeaderValue}};
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

use config::{AppConfig, AppState};
use error::{AppError, AppResult};
use routes::create_router;

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
    let app = create_app(app_state.clone());

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
fn create_app(state: AppState) -> Router {
    // CORS configuration untuk development dan production
    let cors = CorsLayer::new()
        .allow_origin("http://localhost:3000".parse::<HeaderValue>().unwrap()) // Frontend URL
        .allow_origin("http://localhost:3001".parse::<HeaderValue>().unwrap()) // Auth service
        .allow_origin("http://localhost:3002".parse::<HeaderValue>().unwrap()) // User service
        .allow_origin("http://localhost:3003".parse::<HeaderValue>().unwrap()) // Vehicle service
        .allow_origin("http://localhost:3004".parse::<HeaderValue>().unwrap()) // Booking service
        .allow_origin("http://localhost:8080".parse::<HeaderValue>().unwrap()) // Alternative frontend port
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::DELETE,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::ACCEPT,
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
        ])
        .allow_credentials(true);

    // Build router dengan semua middleware
    Router::new()
        .merge(create_router())
        .fallback(not_found_handler)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CompressionLayer::new())
                .layer(cors)
        )
        .with_state(state)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_connection_string() {
        let conn = "postgresql://user:password@localhost:5432/db";
        let masked = mask_connection_string(conn);
        assert_eq!(masked, "postgresql://user:****@localhost:5432/db");

        let conn = "postgresql://postgres.movyypzgmhfuopdgtlup:Syah_aril987@aws-1-us-east-2.pooler.supabase.com:6543/postgres";
        let masked = mask_connection_string(conn);
        assert!(masked.contains("postgres.movyypzgmhfuopdgtlup:****@"));
    }
}
