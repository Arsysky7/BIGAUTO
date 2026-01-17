use axum::{extract::{Path, State, Multipart}, Json};
use shared::utils::cloudinary::CloudinaryClient;
use sqlx::PgPool;

use crate::{
    domain::vehicle::VehicleResponse,
    error::AppError,
    middleware::auth::AuthSeller,
    repositories::vehicle_repo,
};

use super::vehicles::map_to_response;

const MAX_PHOTOS: usize = 10;
const MAX_FILE_SIZE: usize = 5 * 1024 * 1024;

// Upload additional photos
#[utoipa::path(
    post,
    path = "/api/vehicles/{id}/photos",
    tag = "Photos",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "Vehicle ID")),
    responses(
        (status = 200, description = "Photos uploaded", body = VehicleResponse),
        (status = 400, description = "Bad request"),
        (status = 403, description = "Forbidden"),
    )
)]
pub async fn upload_photos(
    auth: AuthSeller,
    Path(id): Path<i32>,
    State(pool): State<PgPool>,
    multipart: Multipart,
) -> Result<Json<VehicleResponse>, AppError> {
    // Audit log: siapa yang upload photos
    tracing::info!(
        "Seller {} ({}) uploading photos for vehicle {}",
        auth.user_id,
        auth.email,
        id
    );

    let vehicle = vehicle_repo::check_ownership(&pool, id, auth.user_id).await?;

    let mut photos: Vec<String> = serde_json::from_value(vehicle.photos.clone())
        .unwrap_or_default();

    if photos.len() >= MAX_PHOTOS {
        return Err(AppError::validation("Maksimal 10 photos per vehicle"));
    }

    let cloudinary = CloudinaryClient::new()
        .map_err(|e| AppError::cloudinary(format!("Cloudinary init error: {}", e)))?;

    let uploaded = process_photo_uploads(multipart, &cloudinary, id, &mut photos).await?;

    if uploaded == 0 {
        return Err(AppError::validation("Tidak ada photo yang diupload"));
    }

    let vehicle = vehicle_repo::update_photos(&pool, id, photos).await?;
    let seller_name = vehicle_repo::find_seller_name(&pool, auth.user_id).await?;

    Ok(Json(map_to_response(vehicle, seller_name)))
}

// Delete specific photo
#[utoipa::path(
    delete,
    path = "/api/vehicles/{id}/photos/{index}",
    tag = "Photos",
    security(("bearer_auth" = [])),
    params(
        ("id" = i32, Path, description = "Vehicle ID"),
        ("index" = usize, Path, description = "Photo index")
    ),
    responses(
        (status = 200, description = "Photo deleted", body = VehicleResponse),
        (status = 400, description = "Bad request"),
        (status = 403, description = "Forbidden"),
    )
)]
pub async fn delete_photo(
    auth: AuthSeller,
    Path((id, index)): Path<(i32, usize)>,
    State(pool): State<PgPool>,
) -> Result<Json<VehicleResponse>, AppError> {
    // Audit log: siapa yang delete photo
    tracing::info!(
        "Seller {} ({}) deleting photo {} from vehicle {}",
        auth.user_id,
        auth.email,
        index,
        id
    );

    let vehicle = vehicle_repo::check_ownership(&pool, id, auth.user_id).await?;

    let mut photos: Vec<String> = serde_json::from_value(vehicle.photos.clone())
        .unwrap_or_default();

    if index >= photos.len() {
        return Err(AppError::validation("Index photo tidak valid"));
    }

    let min_photos = if vehicle.category == "sale" { 5 } else { 3 };
    if photos.len() <= min_photos {
        return Err(AppError::validation(format!(
            "Minimal {} photos untuk category {}",
            min_photos, vehicle.category
        )));
    }

    photos.remove(index);

    let vehicle = vehicle_repo::update_photos(&pool, id, photos).await?;
    let seller_name = vehicle_repo::find_seller_name(&pool, auth.user_id).await?;

    Ok(Json(map_to_response(vehicle, seller_name)))
}

// Process multipart photo uploads
async fn process_photo_uploads(
    mut multipart: Multipart,
    cloudinary: &CloudinaryClient,
    vehicle_id: i32,
    photos: &mut Vec<String>,
) -> Result<usize, AppError> {
    let mut count = 0;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError::bad_request(format!("Multipart error: {}", e))
    })? {
        if field.name() != Some("photos") {
            continue;
        }

        if photos.len() >= MAX_PHOTOS {
            break;
        }

        let data = field.bytes().await.map_err(|e| {
            AppError::bad_request(format!("Read error: {}", e))
        })?;

        if data.len() > MAX_FILE_SIZE {
            return Err(AppError::validation("File maksimal 5MB"));
        }

        let filename = format!("vehicle-{}-{}", vehicle_id, photos.len());
        let result = cloudinary
            .upload_image(data.to_vec(), "vehicles", Some(filename))
            .await
            .map_err(|e| AppError::cloudinary(format!("Upload error: {}", e)))?;

        photos.push(result.secure_url);
        count += 1;
    }

    Ok(count)
}
