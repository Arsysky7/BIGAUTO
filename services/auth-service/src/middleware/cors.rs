// Manual CORS Middleware

use axum::{
    extract::Request,
    http::{header::HeaderName, HeaderValue, Method, StatusCode},
    middleware::Next,
    response::Response,
};

/// Manual CORS middleware
pub async fn manual_cors_middleware(
    request: Request,
    next: Next,
) -> Result<Response, std::convert::Infallible> {
    let method = request.method().clone();
    let origin = request
        .headers()
        .get("origin")
        .and_then(|h| h.to_str().ok());

    // Run the request
    let mut response = next.run(request).await;

    // Add CORS headers ke ALL responses
    let headers = response.headers_mut();

    // Allow all origins
    headers.insert(
        HeaderName::from_static("access-control-allow-origin"),
        HeaderValue::from_static("*"),
    );

    // Allow methods
    let allow_methods = match method.as_str() {
        "OPTIONS" => "GET, POST, PUT, DELETE, OPTIONS",
        _ => "GET, POST, PUT, DELETE, OPTIONS",
    };
    headers.insert(
        HeaderName::from_static("access-control-allow-methods"),
        HeaderValue::from_static(allow_methods),
    );

    // Allow headers
    headers.insert(
        HeaderName::from_static("access-control-allow-headers"),
        HeaderValue::from_static("authorization, content-type, accept"),
    );

    // Handle preflight OPTIONS
    if method == Method::OPTIONS {
        *response.status_mut() = StatusCode::OK;
    }

    Ok(response)
}