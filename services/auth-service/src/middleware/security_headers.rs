// Security Headers untuk Auth Service 

use axum::{
    extract::Request,
    http::{header, HeaderValue},
    middleware::Next,
    response::Response,
};

/// Security Headers Middleware untuk Auth Service
pub async fn security_headers_middleware(
    request: Request,
    next: Next,
) -> Result<Response, std::convert::Infallible> {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    // Environment detection
    let env = std::env::var("RUST_ENV").unwrap_or_else(|_| "development".to_string());

    match env.as_str() {
        "production" => {
            // Production security headers (strict)
            headers.insert(
                header::CONTENT_SECURITY_POLICY,
                "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self' data:".parse().unwrap(),
            );
            headers.insert(
                "X-Frame-Options",
                HeaderValue::from_static("DENY"),
            );
            headers.insert(
                "X-Content-Type-Options",
                HeaderValue::from_static("nosniff"),
            );
            headers.insert(
                header::REFERRER_POLICY,
                HeaderValue::from_static("strict-origin-when-cross-origin"),
            );
            headers.insert(
                "X-XSS-Protection",
                HeaderValue::from_static("1; mode=block"),
            );
            headers.insert(
                header::STRICT_TRANSPORT_SECURITY,
                HeaderValue::from_static("max-age=31536000; includeSubDomains"),
            );
        }
        _ => {
            // Development headers (relaxed)
            headers.insert(
                header::CONTENT_SECURITY_POLICY,
                "default-src 'self' 'unsafe-inline' 'unsafe-eval'; img-src 'self' data: https:".parse().unwrap(),
            );
            headers.insert(
                "X-Frame-Options",
                HeaderValue::from_static("SAMEORIGIN"),
            );
            headers.insert(
                "X-Content-Type-Options",
                HeaderValue::from_static("nosniff"),
            );
        }
    }

    // Common headers untuk semua environment
    headers.insert(
        header::CACHE_CONTROL,
        "no-store, no-cache, must-revalidate, private".parse().unwrap(),
    );
    headers.insert(
        header::PRAGMA,
        "no-cache".parse().unwrap(),
    );

    // Auth Service specific headers
    headers.insert(
        "X-Auth-Service-Version",
        HeaderValue::from_str(&std::env::var("APP_VERSION").unwrap_or_else(|_| "1.0.0".to_string())).unwrap(),
    );
    headers.insert(
        "X-Service-Timestamp",
        HeaderValue::from_str(&chrono::Utc::now().to_rfc3339()).unwrap(),
    );
    headers.insert(
        "X-Auth-Service",
        HeaderValue::from_static("Big Auto Auth Service"),
    );

    // Remove server header untuk security
    headers.remove(header::SERVER);

    Ok(response)
}

