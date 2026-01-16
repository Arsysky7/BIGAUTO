// JWT-Only CORS Configuration untuk Auth Service

use axum::http::{header, HeaderValue, Method};
use tower_http::cors::CorsLayer;

/// Build CORS configuration untuk JWT-Only authentication
/// Supports multiple origins (comma-separated) for development + production
pub fn configure_cors() -> CorsLayer {
    let frontend_urls = std::env::var("FRONTEND_URL")
        .expect("FRONTEND_URL environment variable harus diset");

    // Parse comma-separated origins
    // Example: "https://bigauto.com,http://localhost:5173,http://localhost:3000"
    let allowed_origins: Vec<HeaderValue> = frontend_urls
        .split(',')
        .map(|s| s.trim().parse::<HeaderValue>().expect("Invalid FRONTEND_URL format"))
        .collect();

    let allowed_methods = build_allowed_methods();
    let allowed_headers = build_jwt_headers();

    CorsLayer::new()
        .allow_origin(allowed_origins)  
        .allow_methods(allowed_methods)
        .allow_headers(allowed_headers)
        .allow_credentials(false)
        .max_age(build_cors_max_age())
}

/// Daftar HTTP methods yang diijinkan untuk API
fn build_allowed_methods() -> Vec<Method> {
    vec![
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::DELETE,
        Method::OPTIONS,
    ]
}

/// Headers yang diijinkan untuk JWT-Only authentication
fn build_jwt_headers() -> Vec<header::HeaderName> {
    vec![
        header::AUTHORIZATION,
        header::ACCEPT,      
        header::CONTENT_TYPE,  
    ]
}

/// Max age untuk CORS preflight cache berdasarkan environment
fn build_cors_max_age() -> std::time::Duration {
    let env = std::env::var("RUST_ENV").unwrap_or_else(|_| "development".to_string());

    match env.as_str() {
        "production" => std::time::Duration::from_secs(86400), 
        _ => std::time::Duration::from_secs(3600),             
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cors_configuration() {
        // Test CORS configuration without panicking
        std::env::set_var("FRONTEND_URL", "http://localhost:3000");
        std::env::set_var("RUST_ENV", "test");

        let cors_layer = configure_cors();
        // Just ensure it doesn't panic - actual CORS testing needs integration tests
        assert!(true);
    }
}