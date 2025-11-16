use axum::{routing::{get, post, put, delete}, Json, Router};
use serde::Serialize;
use utoipa::{OpenApi, Modify};
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa_swagger_ui::SwaggerUi;

use crate::config::AppState;
use crate::error::AppResult;
use crate::handlers::{auth, user, session, otp};

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
        description = "Authentication and User Management Service for Big Auto platform\n\n## Features\n\n- 🔐 Email + Password Authentication with 2FA (OTP)\n- ✉️ Email Verification\n- 🔄 JWT Token Refresh\n- 👤 User Profile Management\n- 🏪 Seller Upgrade\n- 📱 Multi-device Session Management\n\n## Authentication Flow\n\n1. **Register**: POST `/api/auth/register` - Create new account\n2. **Verify Email**: GET `/api/auth/verify-email?token={token}` - Verify email from link\n3. **Login Step 1**: POST `/api/auth/login` - Send OTP to email\n4. **Login Step 2**: POST `/api/auth/verify-otp` - Verify OTP and get JWT tokens\n5. **Use Access Token**: Include in `Authorization: Bearer {token}` header\n6. **Refresh Token**: POST `/api/auth/refresh` - Get new access token (uses httpOnly cookie)\n\n## Security\n\n- Passwords hashed with Argon2\n- JWT tokens (15 min access, 7 days refresh)\n- HttpOnly cookies for refresh tokens\n- Rate limiting on OTP requests\n- Session tracking with device info\n",
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

/// Create the main application router with all routes
pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Health check endpoint
        .route("/health", get(health_check))
        // Auth routes (public - tidak perlu JWT)
        .route("/api/auth/register", post(auth::register_handler))
        .route("/api/auth/verify-email", get(auth::verify_email_handler))
        .route("/api/auth/resend-verification", post(auth::resend_verification_handler))
        .route("/api/auth/login", post(auth::login_step1_handler))
        .route("/api/auth/verify-otp", post(auth::login_step2_handler))
        .route("/api/auth/resend-otp", post(auth::resend_otp_handler))
        .route("/api/auth/refresh", post(auth::refresh_token_handler))
        .route("/api/auth/logout", post(auth::logout_handler))
        // OTP status check (protected - perlu JWT)
        .route("/api/auth/otp-status", get(otp::check_otp_status_handler))
        // Session routes (protected - perlu JWT middleware)
        .route("/api/auth/sessions", get(session::get_sessions_handler))
        .route("/api/auth/sessions/{id}", delete(session::invalidate_session_handler))
        .route("/api/auth/sessions/invalidate-all", post(session::invalidate_all_sessions_handler))
        // User routes (protected - perlu JWT middleware)
        .route("/api/users/me", get(user::get_profile_handler))
        .route("/api/users/me", put(user::update_profile_handler))
        .route("/api/users/me/upgrade-seller", post(user::upgrade_to_seller_handler))
        // Swagger UI documentation
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        // Share application state with all routes
        .with_state(state)
}

/// Health check endpoint
///
/// Returns the health status of the service including database and redis connectivity
async fn health_check(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> AppResult<Json<HealthCheckResponse>> {
    let health = state.health_check().await;

    let response = HealthCheckResponse {
        status: health.overall.clone(),
        service: "auth-service".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        database: health.database.clone(),
        redis: health.redis.clone(),
    };

    Ok(Json(response))
}

/// Health check response
#[derive(Debug, Serialize, utoipa::ToSchema)]
struct HealthCheckResponse {
    /// Overall status
    #[schema(example = "healthy")]
    status: String,
    /// Service name
    #[schema(example = "auth-service")]
    service: String,
    /// Service version
    #[schema(example = "0.1.0")]
    version: String,
    /// Database connection status
    #[schema(example = "healthy")]
    database: String,
    /// Redis connection status
    #[schema(example = "healthy")]
    redis: String,
}

