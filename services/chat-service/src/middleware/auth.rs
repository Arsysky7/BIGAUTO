// JWT-Only Authentication Middleware untuk Chat Service

use axum::{
    extract::{Request, State, FromRequestParts},
    http::HeaderMap,
    response::Response,
    middleware::Next,
    http::request::Parts,
    RequestPartsExt,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use sqlx::PgPool;

use crate::{config::AppState, error::AppError, utils::jwt};

// Authenticated user structure
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: i32,
    pub email: String,
    pub role: String,
}

// Axum extractor implementation untuk AuthUser
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| AppError::unauthorized("Authorization header dengan Bearer token diperlukan"))?;

        let claims = jwt::validate_token(bearer.token())
            .map_err(|_| AppError::unauthorized("Token tidak valid atau sudah expired"))?;

        // Validasi role untuk chat service (customer/seller)
        if !claims.is_customer() && !claims.is_seller() {
            return Err(AppError::forbidden("Hanya customer dan seller yang bisa akses chat"));
        }

        tracing::debug!(
            "User authenticated - ID: {}, Email: {}, Role: {}",
            claims.sub,
            claims.email,
            claims.role
        );

        Ok(AuthUser {
            user_id: claims.sub,
            email: claims.email,
            role: claims.role,
        })
    }
}

// Chat participant dengan status check 
#[derive(Debug, Clone)]
pub struct ChatParticipant {
    pub user_id: i32,
    pub email: String,
    pub role: String,
    pub is_active: bool,
}

impl<S> FromRequestParts<S> for ChatParticipant
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let user = AuthUser::from_request_parts(parts, _state).await?;

        // Extract database pool dari request extensions
        let db_pool = parts
            .extensions
            .get::<PgPool>()
            .ok_or_else(|| AppError::internal("Database pool tidak tersedia"))?;

        // Check user status dari database
        let user_status = sqlx::query_scalar!(
            "SELECT is_active FROM users WHERE id = $1",
            user.user_id
        )
        .fetch_one(db_pool)
        .await
        .map_err(|_| AppError::unauthorized("User tidak ditemukan"))?;

        let is_active = user_status.unwrap_or(false);
        if !is_active {
            return Err(AppError::forbidden("User tidak aktif atau dibanned"));
        }

        tracing::debug!(
            "Chat participant {} validated and active",
            user.user_id
        );

        Ok(ChatParticipant {
            user_id: user.user_id,
            email: user.email,
            role: user.role,
            is_active,
        })
    }
}

// WebSocket participant (tanpa blacklist check untuk handshake speed)
#[derive(Debug, Clone)]
pub struct WebSocketParticipant {
    pub user_id: i32,
    pub email: String,
    pub role: String,
    pub is_active: bool,
}

impl<S> FromRequestParts<S> for WebSocketParticipant
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let user = AuthUser::from_request_parts(parts, _state).await?;

        // Extract database pool dari request extensions
        let db_pool = parts
            .extensions
            .get::<PgPool>()
            .ok_or_else(|| AppError::internal("Database pool tidak tersedia"))?;

        // Check user status dari database
        let user_status = sqlx::query_scalar!(
            "SELECT is_active FROM users WHERE id = $1",
            user.user_id
        )
        .fetch_one(db_pool)
        .await
        .map_err(|_| AppError::unauthorized("User tidak ditemukan"))?;

        let is_active = user_status.unwrap_or(false);
        if !is_active {
            return Err(AppError::forbidden("User tidak aktif atau dibanned"));
        }

        tracing::debug!(
            "WebSocket participant {} validated and active",
            user.user_id
        );

        Ok(WebSocketParticipant {
            user_id: user.user_id,
            email: user.email,
            role: user.role,
            is_active,
        })
    }
}

// Extract Bearer token dari Authorization header
fn extract_jwt_token(headers: &HeaderMap) -> Result<String, AppError> {
    let auth_header = headers
        .get("authorization")
        .ok_or_else(|| AppError::unauthorized("Authorization header dengan Bearer token diperlukan"))?
        .to_str()
        .map_err(|_| AppError::unauthorized("Invalid Authorization header format"))?;

    if !auth_header.starts_with("Bearer ") {
        return Err(AppError::unauthorized("Bearer token format diperlukan"));
    }

    Ok(auth_header[7..].to_string())
}

// JWT authentication middleware dengan blacklist validation (HTTP endpoints)
pub async fn jwt_auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Skip authentication untuk health check dan documentation
    let path = request.uri().path().to_string();
    if path == "/health" || path.starts_with("/docs") || path.starts_with("/api-docs") || path.starts_with("/redoc") {
        return Ok(next.run(request).await);
    }

    // Extract JWT token dari Authorization header
    let token = extract_jwt_token(request.headers())?;

    // Validasi JWT dengan database trust boundary (termasuk blacklist check)
    let claims = jwt::validate_token_with_blacklist(&token, &state.db)
        .await
        .map_err(|_| AppError::unauthorized("Token tidak valid, expired, atau sudah di-blacklist"))?;

    // Validasi role untuk chat service
    if !claims.can_access_chat() {
        return Err(AppError::forbidden("Role tidak valid untuk chat service"));
    }

    // Prepare user context untuk handlers
    let auth_user = AuthUser {
        user_id: claims.sub,
        email: claims.email.clone(),
        role: claims.role.clone(),
    };

    // Inject ke request extensions agar bisa di-extract oleh handlers
    request.extensions_mut().insert(auth_user.clone());

    // Security audit log
    tracing::debug!(
        "User authenticated - ID: {}, Email: {}, Role: {}, Endpoint: {}",
        auth_user.user_id,
        auth_user.email,
        auth_user.role,
        path
    );

    Ok(next.run(request).await)
}

// Helper functions untuk role validation
impl AuthUser {
    pub fn is_customer(&self) -> bool {
        self.role == "customer"
    }

    pub fn can_access_conversation(&self, conversation_customer_id: i32, conversation_seller_id: i32) -> bool {
        self.user_id == conversation_customer_id || self.user_id == conversation_seller_id
    }

    pub fn get_conversation_role(&self, conversation_customer_id: i32) -> &'static str {
        if self.user_id == conversation_customer_id {
            "customer"
        } else {
            "seller"
        }
    }
}

// Helper functions untuk participant validation
impl ChatParticipant {
    pub fn is_customer(&self) -> bool {
        self.role == "customer"
    }

    pub fn is_seller(&self) -> bool {
        self.role == "seller"
    }
}