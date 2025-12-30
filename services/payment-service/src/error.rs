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

// Enum untuk semua jenis error yang mungkin terjadi di payment service
#[derive(Debug)]
pub enum AppError {
    DatabaseError(sqlx::Error),
    ValidationError(String),
    UnauthorizedError(String),
    ForbiddenError(String),
    NotFoundError(String),
    PaymentError(String),
    MidtransError(String),
    RefundError(String),
    InternalError(String),
    TokenError(String),
    HttpClientError(reqwest::Error),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::DatabaseError(e) => write!(f, "Database error: {}", e),
            AppError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            AppError::UnauthorizedError(msg) => write!(f, "Unauthorized error: {}", msg),
            AppError::ForbiddenError(msg) => write!(f, "Forbidden error: {}", msg),
            AppError::NotFoundError(msg) => write!(f, "Not found: {}", msg),
            AppError::PaymentError(msg) => write!(f, "Payment error: {}", msg),
            AppError::MidtransError(msg) => write!(f, "Midtrans error: {}", msg),
            AppError::RefundError(msg) => write!(f, "Refund error: {}", msg),
            AppError::InternalError(msg) => write!(f, "Internal error: {}", msg),
            AppError::TokenError(msg) => write!(f, "Token error: {}", msg),
            AppError::HttpClientError(e) => write!(f, "HTTP client error: {}", e),
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

// Konversi dari jsonwebtoken::errors::Error ke AppError
impl From<jsonwebtoken::errors::Error> for AppError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        AppError::TokenError(err.to_string())
    }
}

// Konversi dari reqwest::Error ke AppError
impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        AppError::HttpClientError(err)
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
            AppError::ValidationError(msg) => (
                StatusCode::BAD_REQUEST,
                "validation_error",
                msg.as_str(),
                None,
            ),
            AppError::UnauthorizedError(msg) => (
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                msg.as_str(),
                None,
            ),
            AppError::ForbiddenError(msg) => (
                StatusCode::FORBIDDEN,
                "forbidden",
                msg.as_str(),
                None,
            ),
            AppError::NotFoundError(msg) => {
                (StatusCode::NOT_FOUND, "not_found", msg.as_str(), None)
            }
            AppError::PaymentError(msg) => {
                tracing::error!("Payment error: {}", msg);
                (
                    StatusCode::BAD_REQUEST,
                    "payment_error",
                    msg.as_str(),
                    if cfg!(debug_assertions) {
                        Some(msg.clone())
                    } else {
                        None
                    },
                )
            }
            AppError::MidtransError(msg) => {
                tracing::error!("Midtrans error: {}", msg);
                (
                    StatusCode::BAD_GATEWAY,
                    "payment_gateway_error",
                    "Terjadi kesalahan pada payment gateway",
                    if cfg!(debug_assertions) {
                        Some(msg.clone())
                    } else {
                        None
                    },
                )
            }
            
            AppError::RefundError(msg) => {
                tracing::error!("Refund error: {}", msg);
                (
                    StatusCode::BAD_REQUEST,
                    "refund_error",
                    msg.as_str(),
                    if cfg!(debug_assertions) {
                        Some(msg.clone())
                    } else {
                        None
                    },
                )
            }
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
            AppError::HttpClientError(e) => {
                tracing::error!("HTTP client error: {:?}", e);
                (
                    StatusCode::BAD_GATEWAY,
                    "http_client_error",
                    "Terjadi kesalahan komunikasi dengan external service",
                    if cfg!(debug_assertions) {
                        Some(e.to_string())
                    } else {
                        None
                    },
                )
            }
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

    // Buat error not found dengan pesan custom
    pub fn not_found(msg: impl Into<String>) -> Self {
        AppError::NotFoundError(msg.into())
    }

    // Buat error payment dengan pesan custom
    pub fn payment(msg: impl Into<String>) -> Self {
        AppError::PaymentError(msg.into())
    }

    // Buat error Midtrans dengan pesan custom
    pub fn midtrans(msg: impl Into<String>) -> Self {
        AppError::MidtransError(msg.into())
    }

    // Buat error refund dengan pesan custom
    pub fn refund(msg: impl Into<String>) -> Self {
        AppError::RefundError(msg.into())
    }

    // Buat error internal dengan pesan custom
    pub fn internal(msg: impl Into<String>) -> Self {
        AppError::InternalError(msg.into())
    }

    // Buat error bad request dengan pesan custom
    pub fn bad_request(msg: impl Into<String>) -> Self {
        AppError::ValidationError(msg.into())
    }

    // Buat error database dengan pesan custom
    pub fn database(msg: impl Into<String>) -> Self {
        AppError::DatabaseError(sqlx::Error::Protocol(format!("Database error: {}", msg.into())))
    }

    // Buat error unauthorized dengan pesan custom
    pub fn unauthorized(msg: impl Into<String>) -> Self {
        AppError::UnauthorizedError(msg.into())
    }

    // Buat error forbidden dengan pesan custom
    pub fn forbidden(msg: impl Into<String>) -> Self {
        AppError::ForbiddenError(msg.into())
    }

    // Buat error internal server error dengan pesan custom
    pub fn internal_error(msg: impl Into<String>) -> Self {
        AppError::InternalError(msg.into())
    }
}

// Type alias untuk Result dengan AppError sebagai error type
pub type AppResult<T> = Result<T, AppError>;