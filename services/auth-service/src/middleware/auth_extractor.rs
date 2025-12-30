// JWT-Only Auth Extractor Middleware for Auth Service

use axum::{
    extract::{Request, State, FromRequestParts},
    response::Response,
    middleware::Next,
    http::{StatusCode, header, request::Parts},
};
use serde_json::json;

use crate::config::AppState;
use crate::middleware::auth::extract_authenticated_user;
use crate::handlers::user::AuthenticatedUser;

/// Middleware untuk JWT-only authentication
pub async fn jwt_auth_extractor_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, Response> {
    let path = request.uri().path();

    // Skip validation untuk public endpoints
    if should_skip_jwt_validation(path) {
        return Ok(next.run(request).await);
    }

    // Ekstrak dan validasi JWT dengan enterprise security
    let user_data = extract_authenticated_user(request.headers(), &state.config.jwt_secret, &state.db)
        .await
        .map_err(|(status, message)| create_json_error_response(status, &message))?;

    // Tambah user data ke request untuk handler access
    request.extensions_mut().insert(user_data);

    Ok(next.run(request).await)
}


/// Skip JWT validation untuk public API endpoints
fn should_skip_jwt_validation(path: &str) -> bool {
    match path {
        "/health" | "/swagger-ui" | "/api-docs" => true,
        _ if path.starts_with("/swagger-ui/") => true,
        _ if path.starts_with("/api-docs/") => true,
        _ if path.starts_with("/api/auth/register") => true,
        _ if path.starts_with("/api/auth/verify-email") => true,
        _ if path.starts_with("/api/auth/resend-verification") => true,
        _ if path.starts_with("/api/auth/login") => true,
        _ if path.starts_with("/api/auth/verify-otp") => true,
        _ if path.starts_with("/api/auth/resend-otp") => true,
        _ => false,  
    }
}

/// Buat JSON error response dengan format enterprise
fn create_json_error_response(status: StatusCode, message: &str) -> Response {
    let body = json!({
        "error": status.as_str(),
        "message": message,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "request_id": uuid::Uuid::new_v4().to_string()
    });

    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .header("X-Request-ID", uuid::Uuid::new_v4().to_string())
        .body(axum::body::Body::from(body.to_string()))
        .unwrap()
}

// Implement FromRequestParts for Axum handler integration
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        if let Some(user) = parts.extensions.get::<AuthenticatedUser>() {
            Ok(user.clone())
        } else {
            Err(create_json_error_response(StatusCode::UNAUTHORIZED, "Authentication required - JWT token missing or invalid"))
        }
    }
}