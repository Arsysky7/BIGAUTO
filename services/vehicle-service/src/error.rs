use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

// Custom error type untuk vehicle service dengan response standardized
#[derive(Debug)]
pub enum AppError {
    DatabaseError(sqlx::Error),
    NotFound(String),
    Unauthorized(String),
    Forbidden(String),
    BadRequest(String),
    ValidationError(String),
    Cloudinary(String),
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

    pub fn cloudinary(msg: impl Into<String>) -> Self {
        Self::Cloudinary(msg.into())
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self::InternalServer(msg.into())
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
            AppError::Cloudinary(msg) => {
                tracing::error!("Cloudinary error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "cloudinary_error",
                    msg.clone(),
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
