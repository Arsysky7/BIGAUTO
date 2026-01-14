use axum::{
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde_json::json;
use thiserror::Error;

// Error type untuk aplikasi dengan HTTP mapping
#[derive(Debug, Error)]
pub enum AppError {
    // Authentication errors
    #[error("Authentication failed: {0}")]
    AuthenticationError(String),

    #[error("Authorization required: {0}")]
    AuthorizationError(String),

    #[error("Token invalid or expired: {0}")]
    TokenError(String),

    // Validation errors
    #[error("Validation error: {0}")]
    ValidationError(String),

    // Not found errors
    #[error("Resource not found: {0}")]
    NotFoundError(String),

    // Database errors
    #[error("Database error: {0}")]
    DatabaseError(String),
}

// Builder methods untuk error creation yang clean
impl AppError {
    pub fn validation(message: &str) -> Self {
        AppError::ValidationError(message.to_string())
    }

    pub fn not_found(message: &str) -> Self {
        AppError::NotFoundError(message.to_string())
    }

    pub fn token(message: &str) -> Self {
        AppError::TokenError(message.to_string())
    }

    pub fn database(message: &str) -> Self {
        AppError::DatabaseError(message.to_string())
    }
}

// Mapping error ke HTTP response
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_type, message) = match self {
            AppError::AuthenticationError(msg) => (StatusCode::UNAUTHORIZED, "AUTHENTICATION_ERROR", msg),
            AppError::AuthorizationError(msg) => (StatusCode::FORBIDDEN, "AUTHORIZATION_ERROR", msg),
            AppError::TokenError(msg) => (StatusCode::UNAUTHORIZED, "TOKEN_ERROR", msg),
            AppError::ValidationError(msg) => (StatusCode::BAD_REQUEST, "VALIDATION_ERROR", msg),
            AppError::NotFoundError(msg) => (StatusCode::NOT_FOUND, "NOT_FOUND", msg),
            AppError::DatabaseError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, "DATABASE_ERROR", msg),
        };

        tracing::error!("{}: {}", error_type, message);

        let body = json!({
            "success": false,
            "error": error_type,
            "message": message
        });

        (status, Json(body)).into_response()
    }
}

// Implement From trait untuk error conversion
impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::database(&err.to_string())
    }
}

impl From<jsonwebtoken::errors::Error> for AppError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        AppError::token(&err.to_string())
    }
}

impl From<validator::ValidationErrors> for AppError {
    fn from(err: validator::ValidationErrors) -> Self {
        let messages: Vec<String> = err
            .field_errors()
            .into_iter()
            .flat_map(|(field, errors)| {
                errors.iter().map(move |e| {
                    format!("{}: {}", field, e.code.as_ref())
                })
            })
            .collect();
        AppError::validation(&messages.join(", "))
    }
}