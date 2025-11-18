use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

// Type alias untuk Result dengan AppError
pub type AppResult<T = ()> = Result<T, AppError>;

// Custom error type untuk booking service dengan response standardized
#[derive(Debug)]
pub enum AppError {
    DatabaseError(sqlx::Error),
    NotFound(String),
    Forbidden(String),
    BadRequest(String),
    ValidationError(String),
    Conflict(String),
    InternalServer(String),
    InternalError(String),
    NotImplemented(String),
}

// Additional constructor for error messages
impl AppError {
    pub fn database_error(msg: String) -> Self {
        Self::InternalError(msg)
    }
}

impl AppError {
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
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

    pub fn conflict(msg: impl Into<String>) -> Self {
        Self::Conflict(msg.into())
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self::InternalServer(msg.into())
    }

    pub fn not_implemented(msg: impl Into<String>) -> Self {
        Self::NotImplemented(msg.into())
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
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, "data_tidak_ditemukan", msg.clone()),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, "akses_dilarang", msg.clone()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "request_tidak_valid", msg.clone()),
            AppError::ValidationError(msg) => {
                tracing::warn!("Validation error: {}", msg);
                (StatusCode::UNPROCESSABLE_ENTITY, "validasi_gagal", msg.clone())
            },
            AppError::Conflict(msg) => {
                tracing::warn!("Conflict error: {}", msg);
                (StatusCode::CONFLICT, "konflik", msg.clone())
            },
            AppError::InternalServer(msg) => {
                tracing::error!("Internal server error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "kesalahan_server_internal",
                    msg.clone(),
                )
            },
            AppError::InternalError(msg) => {
                tracing::error!("Internal error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "kesalahan_internal",
                    msg.clone(),
                )
            },
            AppError::NotImplemented(msg) => {
                tracing::warn!("Feature not implemented: {}", msg);
                (
                    StatusCode::NOT_IMPLEMENTED,
                    "belum_diimplementasi",
                    msg.clone(),
                )
            },
        };

        let body = Json(json!({
            "error": error_type,
            "pesan": message,
        }));

        (status, body).into_response()
    }
}
