use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::fmt;

// Struktur response error yang konsisten untuk semua endpoint
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

// Enum untuk semua jenis error yang mungkin terjadi di aplikasi
#[derive(Debug)]
pub enum AppError {
    DatabaseError(sqlx::Error),
    RedisError(redis::RedisError),
    ValidationError(String),
    AuthenticationError(String),
    AuthorizationError(String),
    NotFoundError(String),
    ConflictError(String),
    RateLimitError(String),
    InternalError(String),
    EmailError(String),
    TokenError(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::DatabaseError(e) => write!(f, "Database error: {}", e),
            AppError::RedisError(e) => write!(f, "Redis error: {}", e),
            AppError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            AppError::AuthenticationError(msg) => write!(f, "Authentication error: {}", msg),
            AppError::AuthorizationError(msg) => write!(f, "Authorization error: {}", msg),
            AppError::NotFoundError(msg) => write!(f, "Not found: {}", msg),
            AppError::ConflictError(msg) => write!(f, "Conflict: {}", msg),
            AppError::RateLimitError(msg) => write!(f, "Rate limit exceeded: {}", msg),
            AppError::InternalError(msg) => write!(f, "Internal error: {}", msg),
            AppError::EmailError(msg) => write!(f, "Email error: {}", msg),
            AppError::TokenError(msg) => write!(f, "Token error: {}", msg),
        }
    }
}

impl std::error::Error for AppError {}

// Konversi dari sqlx::Error ke AppError
impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::DatabaseError(err)
    }
}

// Konversi dari redis::RedisError ke AppError
impl From<redis::RedisError> for AppError {
    fn from(err: redis::RedisError) -> Self {
        AppError::RedisError(err)
    }
}

// Konversi dari jsonwebtoken::errors::Error ke AppError
impl From<jsonwebtoken::errors::Error> for AppError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        AppError::TokenError(err.to_string())
    }
}

// Implementasi IntoResponse untuk AppError agar bisa langsung digunakan sebagai response di axum
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_type, message, details) = match &self {
            AppError::DatabaseError(e) => {
                tracing::error!("Database error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "database_error",
                    "Terjadi kesalahan pada database",
                    if cfg!(debug_assertions) {
                        Some(e.to_string())
                    } else {
                        None
                    },
                )
            }
            AppError::RedisError(e) => {
                tracing::error!("Redis error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "cache_error",
                    "Terjadi kesalahan pada sistem cache",
                    if cfg!(debug_assertions) {
                        Some(e.to_string())
                    } else {
                        None
                    },
                )
            }
            AppError::ValidationError(msg) => (
                StatusCode::BAD_REQUEST,
                "validation_error",
                msg.as_str(),
                None,
            ),
            AppError::AuthenticationError(msg) => (
                StatusCode::UNAUTHORIZED,
                "authentication_error",
                msg.as_str(),
                None,
            ),
            AppError::AuthorizationError(msg) => (
                StatusCode::FORBIDDEN,
                "authorization_error",
                msg.as_str(),
                None,
            ),
            AppError::NotFoundError(msg) => {
                (StatusCode::NOT_FOUND, "not_found", msg.as_str(), None)
            }
            AppError::ConflictError(msg) => {
                (StatusCode::CONFLICT, "conflict", msg.as_str(), None)
            }
            AppError::RateLimitError(msg) => (
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limit_exceeded",
                msg.as_str(),
                None,
            ),
            AppError::InternalError(msg) => {
                tracing::error!("Internal error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "Terjadi kesalahan internal server",
                    if cfg!(debug_assertions) {
                        Some(msg.clone())
                    } else {
                        None
                    },
                )
            }
            AppError::EmailError(msg) => {
                tracing::error!("Email error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "email_error",
                    "Gagal mengirim email. Silakan coba lagi atau hubungi support.",
                    if cfg!(debug_assertions) {
                        Some(msg.clone())
                    } else {
                        None
                    },
                )
            }
            AppError::TokenError(msg) => (
                StatusCode::UNAUTHORIZED,
                "token_error",
                "Token tidak valid atau sudah kadaluarsa",
                if cfg!(debug_assertions) {
                    Some(msg.clone())
                } else {
                    None
                },
            ),
        };

        let error_response = ErrorResponse {
            error: error_type.to_string(),
            message: message.to_string(),
            details,
        };

        (status, Json(error_response)).into_response()
    }
}

// Helper functions untuk membuat error dengan mudah
impl AppError {
    // Buat error validasi dengan pesan custom
    pub fn validation(msg: impl Into<String>) -> Self {
        AppError::ValidationError(msg.into())
    }

    // Buat error authentication dengan pesan custom
    pub fn authentication(msg: impl Into<String>) -> Self {
        AppError::AuthenticationError(msg.into())
    }

    // Buat error authorization dengan pesan custom
    pub fn authorization(msg: impl Into<String>) -> Self {
        AppError::AuthorizationError(msg.into())
    }

    // Buat error conflict dengan pesan custom
    pub fn conflict(msg: impl Into<String>) -> Self {
        AppError::ConflictError(msg.into())
    }

    // Buat error rate limit dengan pesan custom
    pub fn rate_limit(msg: impl Into<String>) -> Self {
        AppError::RateLimitError(msg.into())
    }

    // Buat error internal dengan pesan custom
    pub fn internal(msg: impl Into<String>) -> Self {
        AppError::InternalError(msg.into())
    }

    // Buat error email dengan pesan custom (untuk error handling email)
    pub fn email(msg: impl Into<String>) -> Self {
        AppError::EmailError(msg.into())
    }
}
// Type alias untuk Result dengan AppError sebagai error type
pub type AppResult<T> = Result<T, AppError>;
