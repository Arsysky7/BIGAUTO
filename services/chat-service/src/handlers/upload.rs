// Upload Handler untuk Chat Service - Media Files
use axum::{
    extract::{State},
    response::Json,
};
use axum_extra::extract::Multipart;
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use shared::utils::cloudinary::CloudinaryClient;

use crate::{
    config::AppState,
    middleware::ChatParticipant,
    error::AppError,
};

// Constants untuk file upload validation
const MAX_FILES: usize = 5; 
const MAX_FILE_SIZE: usize = 5 * 1024 * 1024; 
const ALLOWED_IMAGE_TYPES: &[&str] = &[
    "image/jpeg", "image/jpg", "image/png", "image/gif", "image/webp"
];
const ALLOWED_DOCUMENT_TYPES: &[&str] = &[
    "application/pdf", "application/msword",
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    "text/plain", "text/csv"
];

// Response untuk upload success
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UploadResponse {
    pub success: bool,
    pub files: Vec<UploadedFile>,
    pub message: String,
}

// Response untuk single file
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UploadedFile {
    pub filename: String,
    pub original_name: Option<String>,
    pub file_type: String,
    pub file_size: usize,
    pub url: String,
    pub thumbnail_url: Option<String>,
    pub category: FileCategory,
}

// Kategori file yang diupload
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum FileCategory {
    Image,
    Document,
}

// Validasi tipe file
fn validate_file_type(content_type: &str) -> Result<FileCategory, AppError> {
    if ALLOWED_IMAGE_TYPES.contains(&content_type) {
        Ok(FileCategory::Image)
    } else if ALLOWED_DOCUMENT_TYPES.contains(&content_type) {
        Ok(FileCategory::Document)
    } else {
        Err(AppError::validation(format!(
            "Tipe file tidak diizinkan: {}. Allowed: images (jpeg, png, gif, webp) dan documents (pdf, doc, docx, txt, csv)",
            content_type
        )))
    }
}

// Generate filename yang unik untuk chat
fn generate_chat_filename(user_id: i32, original_name: &str, index: usize) -> String {
    let timestamp = chrono::Utc::now().timestamp();
    format!("chat-{}-{}-{}-{}", user_id, timestamp, index,
            original_name.trim().chars().take(20).collect::<String>())
}

// Upload multiple files untuk chat
#[utoipa::path(
    post,
    path = "/upload",
    tag = "upload",
    security(("bearer_auth" = [])),
    request_body(
        description = "Multipart form data dengan files untuk upload",
        content_type = "multipart/form-data"
    ),
    responses(
        (status = 200, description = "Files berhasil diupload", body = UploadResponse),
        (status = 400, description = "Request tidak valid atau file terlalu besar"),
        (status = 401, description = "Unauthorized"),
        (status = 429, description = "Rate limit exceeded"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn upload_file(
    State(state): State<AppState>,
    participant: ChatParticipant,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, AppError> {
    tracing::info!("User {} ({}) memulai upload file chat", participant.user_id, participant.email);

    // Validate user rate limit sebelum upload
    let rate_limit_key = format!("upload:{}", participant.user_id);
    let rate_check = sqlx::query_scalar!(
        "SELECT (check_rate_limit($1, 'user', 'file_upload', '/upload', 'chat-service', 60, 5)->>'allowed')::boolean",
        rate_limit_key
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Rate limit check failed for user {}: {}", participant.user_id, e);
        AppError::internal("Rate limit validation failed")
    })?;

    if !rate_check.unwrap_or(false) {
        return Err(AppError::rate_limit("Too many upload attempts. Please wait before trying again."));
    }

    let cloudinary = CloudinaryClient::new()
        .map_err(|e| AppError::cloudinary(format!("Cloudinary init error: {}", e)))?;

    let mut uploaded_files: Vec<UploadedFile> = Vec::new();
    let mut file_count = 0;

    // Process semua fields di multipart
    while let Some(field) = multipart.next_field().await
        .map_err(|e| AppError::bad_request(format!("Multipart error: {}", e)))? {

        // Skip field yang bukan file
        if field.file_name().is_none() {
            continue;
        }

        if file_count >= MAX_FILES {
            return Err(AppError::validation(format!(
                "Maksimal {} files per request", MAX_FILES
            )));
        }

        // Ambil file data
        let file_name = field.file_name()
            .unwrap_or("unknown")
            .to_string();

        let content_type = field.content_type()
            .unwrap_or("application/octet-stream")
            .to_string();

        let data = field.bytes().await
            .map_err(|e| AppError::bad_request(format!("Read file error: {}", e)))?;

        // Validasi file size
        if data.len() > MAX_FILE_SIZE {
            return Err(AppError::validation(format!(
                "File {} terlalu besar. Maksimal {}MB",
                file_name,
                MAX_FILE_SIZE / (1024 * 1024)
            )));
        }

        // Validasi file type
        let file_category = validate_file_type(&content_type)?;

        // Generate filename yang unik
        let safe_filename = generate_chat_filename(participant.user_id, &file_name, file_count);

        // Upload ke Cloudinary dengan folder yang sesuai
        let folder_name = match file_category {
            FileCategory::Image => "chat/images",
            FileCategory::Document => "chat/documents",
        };

        let upload_result = cloudinary
            .upload_image(data.to_vec(), folder_name, Some(safe_filename.clone()))
            .await
            .map_err(|e| AppError::cloudinary(format!("Upload gagal: {}", e)))?;

        // Generate thumbnail untuk images
        let thumbnail_url = if matches!(file_category, FileCategory::Image) {
            Some(format!("{}?w=200&h=200&c_thumb", upload_result.secure_url))
        } else {
            None
        };

        // Tambahkan ke hasil upload
        uploaded_files.push(UploadedFile {
            filename: safe_filename,
            original_name: Some(file_name),
            file_type: content_type,
            file_size: data.len(),
            url: upload_result.secure_url,
            thumbnail_url,
            category: file_category,
        });

        file_count += 1;
        tracing::info!("File {} berhasil diupload untuk user {}",
                      uploaded_files.last().unwrap().filename, participant.user_id);

        // Log file upload ke audit trail
        if let Err(e) = sqlx::query!(
            r#"
            INSERT INTO audit_logs (user_id, ip_address, action, entity_type, entity_id, new_values, service_name, endpoint, http_method, created_at)
            VALUES ($1, '127.0.0.1'::INET, 'FILE_UPLOAD', 'uploaded_file', $2, $3, 'chat-service', '/upload', 'POST', NOW())
            "#,
            participant.user_id,
            uploaded_files.last().unwrap().file_size as i32,
            json!({
                "filename": uploaded_files.last().unwrap().filename,
                "size": uploaded_files.last().unwrap().file_size,
                "type": uploaded_files.last().unwrap().file_type
            })
        )
        .execute(&state.db)
        .await {
            tracing::warn!("Failed to log file upload to audit trail: {}", e);
        }
    }

    // Validasi minimal ada file yang diupload
    if uploaded_files.is_empty() {
        return Err(AppError::validation("Tidak ada file yang diupload"));
    }

    tracing::info!("User {} berhasil mengupload {} files",
                   participant.user_id, uploaded_files.len());

    Ok(Json(UploadResponse {
        success: true,
        files: uploaded_files,
        message: format!("{} files berhasil diupload", file_count),
    }))
}

// Utility function untuk extract file info dari upload response
pub fn extract_file_info_for_message(upload_response: &UploadResponse) -> Vec<(String, Option<String>)> {
    upload_response.files
        .iter()
        .map(|file| (file.url.clone(), file.thumbnail_url.clone()))
        .collect()
}

// Validasi multiple files untuk chat message
pub fn validate_chat_files(files: &[String]) -> Result<(), AppError> {
    if files.is_empty() {
        return Ok(());
    }

    if files.len() > 5 {
        return Err(AppError::validation("Maksimal 5 files per message"));
    }

    // Validasi setiap URL format
    for file_url in files {
        if !file_url.starts_with("https://res.cloudinary.com") {
            return Err(AppError::validation("URL file tidak valid"));
        }
    }

    Ok(())
}

// Generate preview text untuk message dengan files
pub fn generate_preview_text(files: &[UploadedFile]) -> String {
    if files.is_empty() {
        return String::new();
    }

    let image_count = files.iter().filter(|f| matches!(f.category, FileCategory::Image)).count();
    let doc_count = files.iter().filter(|f| matches!(f.category, FileCategory::Document)).count();

    match (image_count, doc_count) {
        (0, 0) => "File terlampir".to_string(),
        (img, 0) if img == 1 => "📷 Gambar".to_string(),
        (img, 0) => format!("📷 {} gambar", img),
        (0, doc) if doc == 1 => "📄 Dokumen".to_string(),
        (0, doc) => format!("📄 {} dokumen", doc),
        (img, doc) => format!("📷 {} gambar, 📄 {} dokumen", img, doc),
    }
}