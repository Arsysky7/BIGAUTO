use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::{
    config::AppState,
    domain::user::{self as user_domain, ProfileResponse, UpdateProfileInput, UpgradeToSellerInput},
    error::AppResult,
    middleware::auth::extract_authenticated_user,
};

// ===== AUTH EXTRACTOR =====

// Struct untuk menyimpan data user yang sudah terautentikasi dari JWT middleware
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: i32,
    pub email: String,
    pub is_customer: bool,
    pub is_seller: bool,
}

// ===== REQUEST DTOs =====

/// Request body untuk update profile
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateProfileRequestBody {
    /// Nama lengkap (opsional)
    #[schema(example = "John Doe Updated")]
    pub name: Option<String>,
    /// Nomor telepon (opsional)
    #[schema(example = "+6281234567890")]
    pub phone: Option<String>,
    /// Alamat lengkap (opsional)
    #[schema(example = "Jl. Sudirman No. 456")]
    pub address: Option<String>,
    /// Kota (opsional)
    #[schema(example = "Bandung")]
    pub city: Option<String>,
    /// URL profile photo (opsional)
    #[schema(example = "https://example.com/photos/profile.jpg")]
    pub profile_photo: Option<String>,
}

/// Request body untuk upgrade to seller
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpgradeToSellerRequestBody {
    /// Nama business untuk seller
    #[schema(example = "Toko Mobil Sejahtera")]
    pub business_name: String,
}

// ===== HANDLER FUNCTIONS =====

/// Get current user profile
#[utoipa::path(
    get,
    path = "/api/users/me",
    responses(
        (status = 200, description = "Successfully retrieved user profile", body = ProfileResponse),
    ),
    tag = "Users",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_profile_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl IntoResponse> {
    // Extract user dari JWT token
    let auth_user = extract_authenticated_user(&headers, &state.config.jwt_secret, &state.db)
        .await
        .map_err(|(_status, msg)| crate::error::AppError::authentication(&msg))?;

    // Call domain layer untuk get profile
    let profile = user_domain::get_profile(&state, auth_user.user_id).await?;

    Ok(Json(profile))
}

/// Update current user profile
#[utoipa::path(
    put,
    path = "/api/users/me",
    request_body = UpdateProfileRequestBody,
    responses(
        (status = 200, description = "Successfully updated user profile", body = ProfileResponse),
    ),
    tag = "Users",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn update_profile_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<UpdateProfileRequestBody>,
) -> AppResult<impl IntoResponse> {
    // Extract user dari JWT token
    let auth_user = extract_authenticated_user(&headers, &state.config.jwt_secret, &state.db)
        .await
        .map_err(|(_status, msg)| crate::error::AppError::authentication(&msg))?;

    // Convert request body ke domain input
    let input = UpdateProfileInput {
        name: req.name,
        phone: req.phone,
        address: req.address,
        city: req.city,
        profile_photo: req.profile_photo,
    };

    // Call domain layer untuk update profile (includes validation)
    let updated_profile = user_domain::update_profile(&state, auth_user.user_id, input).await?;

    Ok(Json(updated_profile))
}

/// Upgrade user account to seller
#[utoipa::path(
    post,
    path = "/api/users/me/upgrade-seller",
    request_body = UpgradeToSellerRequestBody,
    responses(
        (status = 200, description = "Successfully upgraded to seller", body = ProfileResponse),
    ),
    tag = "Users",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn upgrade_to_seller_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<UpgradeToSellerRequestBody>,
) -> AppResult<impl IntoResponse> {
    // Extract user dari JWT token
    let auth_user = extract_authenticated_user(&headers, &state.config.jwt_secret, &state.db)
        .await
        .map_err(|(_status, msg)| crate::error::AppError::authentication(&msg))?;

    // Convert request body ke domain input
    let input = UpgradeToSellerInput {
        business_name: req.business_name,
    };

    // Authorization check: HANYA customer yang bisa upgrade ke seller (seller sudah seller)
    if auth_user.is_seller {
        return Err(crate::error::AppError::conflict(
            "Anda sudah terdaftar sebagai seller"
        ));
    }

    // Call domain layer untuk upgrade to seller (includes validation)
    let updated_profile = user_domain::upgrade_to_seller(&state, auth_user.user_id, input).await?;

    // Log activity dengan email dari authenticated user
    tracing::info!(
        "User {} ({}) successfully upgraded to seller",
        auth_user.email,
        auth_user.user_id
    );

    Ok((StatusCode::OK, Json(updated_profile)))
}
