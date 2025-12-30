use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::fmt;

// Custom error type untuk chat service dengan response standardized
#[derive(Debug)]
pub enum AppError {
    DatabaseError(sqlx::Error),
    NotFound(String),
    Unauthorized(String),
    Forbidden(String),
    BadRequest(String),
    ValidationError(String),
    RateLimit(String),
    WebSocket(String),
    NATS(String),
    InternalServer(String),
}

impl AppError {
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }

    pub fn unauthorized(msg: impl Into<String>) -> Self {
        Self::Unauthorized(msg.into())
    }

    pub fn forbidden(msg: impl Into<String>) -> Self {
        Self::Forbidden(msg.into())
    }

    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self::BadRequest(msg.into())
    }

    pub fn validation(msg: impl Into<String>) -> Self {
        Self::ValidationError(msg.into())
    }

    pub fn rate_limit(msg: impl Into<String>) -> Self {
        Self::RateLimit(msg.into())
    }

    pub fn websocket(msg: impl Into<String>) -> Self {
        Self::WebSocket(msg.into())
    }

    pub fn nats(msg: impl Into<String>) -> Self {
        Self::NATS(msg.into())
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self::InternalServer(msg.into())
    }

    pub fn cloudinary(msg: impl Into<String>) -> Self {
        Self::BadRequest(format!("File upload error: {}", msg.into()))
    }
}

// Konversi dari sqlx::Error ke AppError
impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => AppError::NotFound("Data tidak ditemukan".to_string()),
            _ => {
                tracing::error!("Database error: {:?}", err);
                AppError::DatabaseError(err)
            }
        }
    }
}

// Konversi dari async_nats::Error ke AppError
impl From<async_nats::Error> for AppError {
    fn from(err: async_nats::Error) -> Self {
        tracing::error!("NATS error: {:?}", err);
        AppError::NATS(format!("NATS connection error: {}", err))
    }
}

// Konversi dari axum::Error ke AppError untuk WebSocket
impl From<axum::Error> for AppError {
    fn from(err: axum::Error) -> Self {
        tracing::error!("WebSocket error: {:?}", err);
        AppError::WebSocket(format!("WebSocket error: {}", err))
    }
}

// Implementasi IntoResponse untuk return error sebagai JSON response
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_type, message) = match &self {
            AppError::DatabaseError(err) => {
                // Log detailed error untuk debugging
                tracing::error!("Database error: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "database_error",
                    "Terjadi kesalahan pada database".to_string(),
                )
            },
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, "not_found", msg.clone()),
            AppError::Unauthorized(msg) => (
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                msg.clone(),
            ),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, "forbidden", msg.clone()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "bad_request", msg.clone()),
            AppError::ValidationError(msg) => {
                tracing::warn!("Validation error: {}", msg);
                (StatusCode::UNPROCESSABLE_ENTITY, "validation_error", msg.clone())
            },
            AppError::RateLimit(msg) => {
                tracing::warn!("Rate limit exceeded: {}", msg);
                (StatusCode::TOO_MANY_REQUESTS, "rate_limit", msg.clone())
            },
            AppError::WebSocket(msg) => {
                tracing::error!("WebSocket error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "websocket_error",
                    msg.clone(),
                )
            },
            AppError::NATS(msg) => {
                tracing::error!("NATS error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "nats_error",
                    "Real-time service unavailable".to_string(),
                )
            },
            AppError::InternalServer(msg) => {
                tracing::error!("Internal server error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_server_error",
                    msg.clone(),
                )
            },
        };

        let body = Json(json!({
            "error": error_type,
            "message": message,
        }));

        (status, body).into_response()
    }
}

// Display trait untuk error formatting
impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::DatabaseError(err) => write!(f, "Database error: {}", err),
            AppError::NotFound(msg) => write!(f, "Not found: {}", msg),
            AppError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            AppError::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
            AppError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            AppError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            AppError::RateLimit(msg) => write!(f, "Rate limit exceeded: {}", msg),
            AppError::WebSocket(msg) => write!(f, "WebSocket error: {}", msg),
            AppError::NATS(msg) => write!(f, "NATS error: {}", msg),
            AppError::InternalServer(msg) => write!(f, "Internal server error: {}", msg),
        }
    }
}