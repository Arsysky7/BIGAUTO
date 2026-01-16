use axum::{Router, middleware};
use serde::Serialize;
use utoipa::{OpenApi, Modify};
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa_swagger_ui::SwaggerUi;

use crate::config::AppState;
use crate::middleware::{
    rate_limit::auth_rate_limit_middleware,
    security_headers::security_headers_middleware,
    cors::configure_cors,
};

// Security scheme modifier for Bearer authentication
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build()
                ),
            )
        }
    }
}

// OpenAPI Documentation
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Big Auto - Auth Service API",
        version = "0.1.0",
        description = "JWT-Only Authentication and User Management Service for Big Auto platform\n\n## Features\n\n- 🔐 JWT-Only Authentication with Bearer Tokens\n- ✉️ Email Verification\n- 🔄 JWT Token Refresh with Secure Blacklist Support\n- 👤 User Profile Management\n- 🏪 Seller Upgrade\n- 📱 Multi-device Session Management\n- 🔒 JWT Blacklist for immediate token revocation\n- 🛡️ Enterprise-grade Security\n\n## Authentication Flow\n\n1. **Register**: POST `/api/auth/register` - Create new account\n2. **Verify Email**: GET `/verify-email?token={token}` - Verify email from link\n3. **Login Step 1**: POST `/api/auth/login` - Send OTP to email\n4. **Login Step 2**: POST `/api/auth/verify-otp` - Verify OTP and get JWT tokens\n5. **Use Access Token**: Include in `Authorization: Bearer {token}` header for all authenticated requests\n6. **Refresh Token**: POST `/api/auth/refresh` - Get new access token (uses Bearer token)\n7. **Logout**: POST `/api/auth/logout` - Blacklist JWT and invalidate session (requires JWT)\n\n## Security Features\n\n- Passwords hashed with Argon2\n- JWT tokens with JTI (JWT ID) for secure blacklist support (15 min access, 7 days refresh)\n- Memory-based refresh token storage (no cookies)\n- Secure database function blacklist validation\n- JWT blacklist database for immediate token invalidation\n- Rate limiting per role (Redis-based)\n- Session tracking with device info\n- Security headers and CORS protection\n- Audit logging for security events\n",
    ),
    paths(
        // Auth endpoints
        crate::handlers::auth::register_handler,
        crate::handlers::auth::verify_email_handler,
        crate::handlers::auth::resend_verification_handler,
        crate::handlers::auth::login_step1_handler,
        crate::handlers::auth::login_step2_handler,
        crate::handlers::auth::resend_otp_handler,
        crate::handlers::auth::refresh_token_handler,
        crate::handlers::auth::logout_handler,
        // OTP endpoints
        crate::handlers::otp::check_otp_status_handler,
        // Session endpoints
        crate::handlers::session::get_sessions_handler,
        crate::handlers::session::invalidate_session_handler,
        crate::handlers::session::invalidate_all_sessions_handler,
        // User endpoints
        crate::handlers::user::get_profile_handler,
        crate::handlers::user::update_profile_handler,
        crate::handlers::user::upgrade_to_seller_handler,
    ),
    modifiers(&SecurityAddon),
    components(
        schemas(
            // Auth DTOs
            crate::handlers::auth::RegisterRequestBody,
            crate::handlers::auth::VerifyEmailQuery,
            crate::handlers::auth::ResendVerificationRequest,
            crate::handlers::auth::LoginRequestBody,
            crate::handlers::auth::VerifyOtpRequestBody,
            crate::handlers::auth::ResendOtpRequest,
            crate::handlers::auth::LogoutRequest,
            crate::handlers::auth::MessageResponse,
            crate::handlers::auth::LoginStep1Response,
            crate::handlers::auth::LoginStep2Response,
            crate::handlers::auth::RefreshTokenResponse,
            crate::domain::auth::UserData,
            crate::domain::auth::RegisterResponse,

            // Session DTOs
            crate::handlers::session::SessionResponse,
            crate::handlers::session::MessageResponse,

            // User DTOs
            crate::handlers::user::UpdateProfileRequestBody,
            crate::handlers::user::UpgradeToSellerRequestBody,
            crate::domain::user::ProfileResponse,

            // OTP DTOs
            crate::handlers::otp::OtpStatusResponse,

            // Health Check
            HealthCheckResponse,
        )
    )
)]
struct ApiDoc;

/// Create public routes
fn create_public_routes(state: AppState) -> Router {
    Router::new()
        // Health check
        .route("/health", axum::routing::get(health_check))

        // Auth endpoints - Public access (no JWT required)
        .route("/api/auth/register", axum::routing::post(crate::handlers::auth::register_handler))
        .route("/verify-email", axum::routing::get(crate::handlers::auth::verify_email_handler))
        .route("/api/auth/resend-verification", axum::routing::post(crate::handlers::auth::resend_verification_handler))
        .route("/api/auth/login", axum::routing::post(crate::handlers::auth::login_step1_handler))
        .route("/api/auth/verify-otp", axum::routing::post(crate::handlers::auth::login_step2_handler))
        .route("/api/auth/resend-otp", axum::routing::post(crate::handlers::auth::resend_otp_handler))
        

        .with_state(state.clone())
}

/// Create JWT-protected routes
fn create_jwt_protected_routes(state: AppState) -> Router {
    Router::new()
        // Refresh token - JWT protection 
        .route("/api/auth/refresh", axum::routing::post(crate::handlers::auth::refresh_token_handler))

        // Logout - JWT protection only
        .route("/api/auth/logout", axum::routing::post(crate::handlers::auth::logout_handler))

        // OTP status check - JWT protection only
        .route("/api/auth/otp-status", axum::routing::get(crate::handlers::otp::check_otp_status_handler))

        // Session endpoints - JWT protection only
        .route("/api/auth/sessions", axum::routing::get(crate::handlers::session::get_sessions_handler))
        .route("/api/auth/sessions/{id}", axum::routing::delete(crate::handlers::session::invalidate_session_handler))
        .route("/api/auth/sessions/invalidate-all", axum::routing::post(crate::handlers::session::invalidate_all_sessions_handler))

        // User endpoints - JWT protection only
        .route("/api/users/me", axum::routing::get(crate::handlers::user::get_profile_handler))
        .route("/api/users/me", axum::routing::put(crate::handlers::user::update_profile_handler))
        .route("/api/users/me/upgrade-seller", axum::routing::post(crate::handlers::user::upgrade_to_seller_handler))

        .with_state(state.clone())
        // Apply JWT middleware untuk protected routes
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::middleware::auth_extractor::jwt_auth_extractor_middleware
        ))
}

/// Health check endpoint
async fn health_check(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> axum::response::Json<serde_json::Value> {
    let health = state.health_check().await;

    axum::response::Json(serde_json::json!({
        "status": health.overall,
        "service": "auth-service",
        "version": env!("CARGO_PKG_VERSION"),
        "database": health.database,
        "redis": health.redis
    }))
}

/// Create the main application router
pub fn create_router(state: AppState) -> Router {
    // Base security layers
    Router::new()
        // CORS
        .layer(configure_cors())
        // Security headers
        // .layer(middleware::from_fn(security_headers_middleware))
        // Rate limiting with proper state
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_rate_limit_middleware
        ))
        // Merge public routes
        .merge(create_public_routes(state.clone()))
        // Merge protected routes
        .merge(create_jwt_protected_routes(state))
        // Add Swagger UI documentation
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
}



/// Health check response
#[derive(Debug, Serialize, utoipa::ToSchema)]
struct HealthCheckResponse {
    #[schema(example = "healthy")]
    status: String,
    #[schema(example = "auth-service")]
    service: String,
    #[schema(example = "0.1.0")]
    version: String,
    #[schema(example = "healthy")]
    database: String,
    #[schema(example = "healthy")]
    redis: String,
}
