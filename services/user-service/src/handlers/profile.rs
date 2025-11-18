use axum::{
    extract::{Path, State, Multipart},
    Json,
};
use serde::Serialize;
use shared::utils::{cloudinary::CloudinaryClient, validation};
use sqlx::{PgPool, Row, FromRow};
use utoipa::ToSchema;

use crate::{
    config::AppConfig,
    domain::user::{User, UserProfile, UpdateProfileRequest, UpgradeToSellerRequest, UploadPhotoResponse},
    error::AppError,
    middleware::AuthUser,
};

// Response sukses dengan message 
#[derive(Debug, Serialize, ToSchema)]
pub struct MessageResponse {
    pub message: String,
}

// Ambil profile user yang sedang login
#[utoipa::path(
    get,
    path = "/api/users/me",
    tag = "Profile",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Profile berhasil diambil", body = UserProfile),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn get_my_profile(
    auth: AuthUser,
    State(pool): State<PgPool>,
) -> Result<Json<UserProfile>, AppError> {
    // Log access untuk audit trail
    tracing::info!(
        "User {} ({}) accessing own profile",
        auth.email,
        auth.role
    );

    let user = find_user_by_id(&pool, auth.user_id).await?;
    Ok(Json(UserProfile::from(user)))
}

// Ambil profile user berdasarkan ID (public)
#[utoipa::path(
    get,
    path = "/api/users/{user_id}",
    tag = "Profile",
    params(
        ("user_id" = i32, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "Profile berhasil diambil", body = UserProfile),
        (status = 404, description = "User tidak ditemukan"),
    )
)]
pub async fn get_user_profile(
    Path(user_id): Path<i32>,
    State(pool): State<PgPool>,
) -> Result<Json<UserProfile>, AppError> {
    let user = find_user_by_id(&pool, user_id).await?;
    Ok(Json(UserProfile::from(user)))
}

// Update profile user
#[utoipa::path(
    put,
    path = "/api/users/me",
    tag = "Profile",
    security(("bearer_auth" = [])),
    request_body = UpdateProfileRequest,
    responses(
        (status = 200, description = "Profile berhasil diupdate", body = UserProfile),
        (status = 400, description = "Input tidak valid"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn update_profile(
    auth: AuthUser,
    State(pool): State<PgPool>,
    Json(payload): Json<UpdateProfileRequest>,
) -> Result<Json<UserProfile>, AppError> {
    // Log untuk audit trail
    tracing::info!(
        "User {} ({}) updating profile",
        auth.email,
        auth.role
    );

    // Validasi input jika ada
    if let Some(ref phone) = payload.phone {
        if !validation::is_valid_phone(phone) {
            return Err(AppError::validation("Format nomor HP tidak valid"));
        }
    }

    let user = update_user_profile(&pool, auth.user_id, payload).await?;
    Ok(Json(UserProfile::from(user)))
}

// Upgrade ke seller
#[utoipa::path(
    post,
    path = "/api/users/me/upgrade-seller",
    tag = "Profile",
    security(("bearer_auth" = [])),
    request_body = UpgradeToSellerRequest,
    responses(
        (status = 200, description = "Berhasil upgrade ke seller", body = UserProfile),
        (status = 400, description = "User sudah menjadi seller"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn upgrade_to_seller(
    auth: AuthUser,
    State(pool): State<PgPool>,
    Json(payload): Json<UpgradeToSellerRequest>,
) -> Result<Json<UserProfile>, AppError> {
    // Log upgrade attempt untuk audit trail
    tracing::info!(
        "User {} ({}) requesting seller upgrade",
        auth.email,
        auth.role
    );

    // Cek apakah sudah seller
    let user = find_user_by_id(&pool, auth.user_id).await?;
    if user.is_seller {
        return Err(AppError::bad_request("User sudah menjadi seller"));
    }

    // Validasi business name tidak kosong
    if payload.business_name.trim().is_empty() {
        return Err(AppError::validation("Nama bisnis tidak boleh kosong"));
    }

    let updated_user = set_user_as_seller(&pool, auth.user_id, &payload.business_name).await?;

    tracing::info!(
        "User {} successfully upgraded to seller",
        auth.email
    );

    Ok(Json(UserProfile::from(updated_user)))
}

// Upload foto profile via Cloudinary
#[utoipa::path(
    post,
    path = "/api/users/me/upload-photo",
    tag = "Profile",
    security(("bearer_auth" = [])),
    request_body(content = String, description = "Multipart form with image file", content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Foto berhasil diupload", body = UploadPhotoResponse),
        (status = 400, description = "File tidak valid"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn upload_profile_photo(
    auth: AuthUser,
    State(pool): State<PgPool>,
    State(config): State<AppConfig>,
    mut multipart: Multipart,
) -> Result<Json<UploadPhotoResponse>, AppError> {
    // Log untuk audit trail
    tracing::info!(
        "User {} ({}) uploading profile photo",
        auth.email,
        auth.role
    );

    // Extract file dari multipart dengan strict validation
    let file_bytes = extract_image_from_multipart(&mut multipart, &config).await?;

    // Upload ke Cloudinary
    let cloudinary = CloudinaryClient::new()
        .map_err(|e| AppError::internal(format!("Cloudnary init failed: {}", e)))?;

    let filename = format!("user-{}", auth.user_id);
    let upload_result = cloudinary
        .upload_image(file_bytes, "profiles", Some(filename))
        .await
        .map_err(|e| AppError::cloudinary(e.to_string()))?;


    // Update database
    update_user_photo(&pool, auth.user_id, &upload_result.secure_url).await?;

    // Generate thumbnail URL
    let thumbnail = cloudinary.thumbnail_url(&upload_result.public_id, Some(150));

    Ok(Json(UploadPhotoResponse {
        url: upload_result.secure_url,
        thumbnail,
    }))
}

// === Helper Functions ===

// Cari user berdasarkan ID
async fn find_user_by_id(pool: &PgPool, user_id: i32) -> Result<User, AppError> {
    let result = sqlx::query(
        "SELECT * FROM users WHERE id = $1 AND is_active = true"
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    match result {
        Some(row) => {
            let user = User::from_row(&row)?;
            Ok(user)
        }
        None => Err(AppError::not_found("User tidak ditemukan")),
    }
}

// Update profile user
async fn update_user_profile(
    pool: &PgPool,
    user_id: i32,
    payload: UpdateProfileRequest,
) -> Result<User, AppError> {
    let result = sqlx::query(
        r#"
        UPDATE users
        SET
            name = COALESCE($2, name),
            phone = COALESCE($3, phone),
            address = COALESCE($4, address),
            city = COALESCE($5, city),
            business_name = COALESCE($6, business_name),
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#
    )
    .bind(user_id)
    .bind(&payload.name)
    .bind(&payload.phone)
    .bind(&payload.address)
    .bind(&payload.city)
    .bind(&payload.business_name)
    .fetch_one(pool)
    .await?;

    let user = User::from_row(&result)?;
    Ok(user)
}

// Set user sebagai seller
async fn set_user_as_seller(
    pool: &PgPool,
    user_id: i32,
    business_name: &str,
) -> Result<User, AppError> {
    let result = sqlx::query(
        r#"
        UPDATE users
        SET
            is_seller = true,
            business_name = $2,
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#
    )
    .bind(user_id)
    .bind(business_name)
    .fetch_one(pool)
    .await?;

    let user = User::from_row(&result)?;
    Ok(user)
}

// Update foto profile user
async fn update_user_photo(
    pool: &PgPool,
    user_id: i32,
    photo_url: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE users
        SET profile_photo = $2, updated_at = NOW()
        WHERE id = $1
        "#
    )
    .bind(user_id)
    .bind(photo_url)
    .execute(pool)
    .await?;

    Ok(())
}

// Extract image bytes dari multipart form dengan strict validation
async fn extract_image_from_multipart(
    multipart: &mut Multipart,
    config: &AppConfig,
) -> Result<Vec<u8>, AppError> {
    // Define max file size based on environment
    let max_size = if config.strict_validation() {
        2 * 1024 * 1024  // Production: 2MB untuk optimasi dan security
    } else {
        5 * 1024 * 1024  // Development: 5MB lebih lenient untuk testing
    };

    let size_label = if config.strict_validation() { "2MB" } else { "5MB" };

    while let Some(field) = multipart.next_field().await
        .map_err(|e| AppError::bad_request(format!("Multipart error: {}", e)))? {

        if field.name() == Some("file") || field.name() == Some("photo") {
            let data = field.bytes().await
                .map_err(|e| AppError::bad_request(format!("Failed to read file: {}", e)))?;

            // Strict validation untuk production environment
            if data.len() > max_size {
                return Err(AppError::validation(format!(
                    "Ukuran file maksimal {} {}",
                    size_label,
                    if config.strict_validation() { "(production mode)" } else { "" }
                )));
            }

            // Log untuk monitoring di production
            if config.is_production() {
                tracing::info!(
                    "Profile photo upload: size={}KB, limit={}MB",
                    data.len() / 1024,
                    max_size / 1024 / 1024
                );
            }

            return Ok(data.to_vec());
        }
    }

    Err(AppError::bad_request("File tidak ditemukan dalam form"))
}
