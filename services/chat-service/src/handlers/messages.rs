// Message Handlers untuk Chat Service
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    config::AppState,
    domain::{Message, MessageType, CreateMessageRequest, MessageResponse},
    middleware::ChatParticipant,
    error::AppError,
    handlers::upload::{validate_chat_files, generate_preview_text, FileCategory, UploadResponse, UploadedFile, extract_file_info_for_message},
};

// Query parameters untuk pagination dan search
#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct MessageQuery {
    pub conversation_id: Option<i32>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub search: Option<String>,
}

// Response untuk message list
#[derive(Debug, Serialize, ToSchema)]
pub struct MessageListResponse {
    pub messages: Vec<Message>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
    pub conversation_id: i32,
}

// Response untuk message count
#[derive(Debug, Serialize, ToSchema)]
pub struct MessageCountResponse {
    pub total_messages: i64,
    pub unread_messages: i64,
    pub conversation_id: i32,
}

// Broadcast message ke NATS untuk real-time delivery
async fn broadcast_message_via_nats(
    nats_client: &async_nats::Client,
    conversation_id: i32,
    message: &Message,
    participant_email: &str,
) -> Result<(), AppError> {
    // Buat payload untuk WebSocket broadcast
    let broadcast_payload = serde_json::json!({
        "type": "new_message",
        "conversation_id": conversation_id,
        "message": {
            "id": message.id,
            "sender_id": message.sender_id,
            "content": message.content,
            "message_type": message.message_type,
            "media_url": message.media_url,
            "thumbnail_url": message.thumbnail_url,
            "created_at": message.created_at,
            "sender_email": participant_email
        }
    });

    // Publish ke NATS subject untuk conversation spesifik
    let subject = format!("chat.{}", conversation_id);
    let payload_str = broadcast_payload.to_string();

    nats_client
        .publish(subject, payload_str.clone().into())
        .await
        .map_err(|e| {
            tracing::error!("Gagal broadcast message ke NATS: {}", e);
            AppError::nats("Gagal mengirim pesan real-time")
        })?;

    // Juga publish ke user-specific subjects untuk targeted delivery
    let user_subject = format!("chat.user.{}", message.sender_id);
    nats_client
        .publish(user_subject, payload_str.into())
        .await
        .map_err(|e| {
            tracing::error!("Gagal broadcast message ke user subject: {}", e);
            AppError::nats("Gagal mengirim pesan real-time")
        })?;

    tracing::info!("Message {} di broadcast ke conversation {} via NATS", message.id, conversation_id);
    Ok(())
}

// Kirim message baru ke conversation
#[utoipa::path(
    post,
    path = "/messages",
    tag = "messages",
    security(("bearer_auth" = [])),
    request_body = CreateMessageRequest,
    responses(
        (status = 201, description = "Message berhasil dikirim", body = Message),
        (status = 400, description = "Request tidak valid"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Tidak memiliki akses"),
        (status = 404, description = "Conversation tidak ditemukan"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn send_message(
    State(state): State<AppState>,
    participant: ChatParticipant,
    Path(conversation_id): Path<i32>,
    Json(request): Json<CreateMessageRequest>,
) -> Result<(StatusCode, Json<MessageResponse>), AppError> {
    // Validasi role participant - customer dan seller bisa kirim message
    if !participant.is_customer() && !participant.is_seller() {
        return Err(AppError::forbidden("Role tidak valid untuk mengirim pesan"));
    }

    // Cek apakah user adalah participant dalam conversation
    let is_participant = state.conversation_repo
        .is_participant(conversation_id, participant.user_id)
        .await?;

    if !is_participant {
        return Err(AppError::forbidden("Tidak memiliki akses ke conversation ini"));
    }

    // Buat message baru
    let message = state.message_repo
        .create_message(conversation_id, participant.user_id, request)
        .await?;

    // Get sender name for MessageResponse
    let sender_name = sqlx::query_scalar!(
        "SELECT name FROM users WHERE id = $1",
        participant.user_id
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or_else(|_| "Unknown".to_string());

    // Update last message info di conversation
    let content_preview = if message.content.len() > 50 {
        format!("{}...", &message.content[..50])
    } else {
        message.content.clone()
    };

    state.conversation_repo
        .update_last_message(conversation_id, &content_preview)
        .await?;

    // Broadcast message via NATS ke WebSocket connections
    if let Some(nats_client) = &state.nats_client {
        if let Err(e) = broadcast_message_via_nats(
            nats_client,
            conversation_id,
            &message,
            &participant.email,
        ).await {
            // Log error tapi tidak gagalkan request, karena message sudah tersimpan di database
            tracing::warn!("Broadcast message gagal: {}, tapi message tersimpan di database", e);
        }
    } else {
        tracing::warn!("NATS client tidak tersedia, message tidak di-broadcast secara real-time");
    }

    tracing::info!("User {} mengirim message {} ke conversation {}",
                   participant.user_id, message.id, conversation_id);

    
    let message_response = message.to_response(sender_name);

    Ok((StatusCode::CREATED, Json(message_response)))
}

// Ambil messages dalam conversation dengan pagination
#[utoipa::path(
    get,
    path = "/messages/conversation/{conversation_id}",
    tag = "messages",
    security(("bearer_auth" = [])),
    params(
        ("conversation_id" = i32, Path, description = "Conversation ID"),
        MessageQuery
    ),
    responses(
        (status = 200, description = "Messages berhasil diambil", body = MessageListResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Tidak memiliki akses ke conversation"),
        (status = 404, description = "Conversation tidak ditemukan"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_conversation_messages(
    State(state): State<AppState>,
    participant: ChatParticipant,
    Path(conversation_id): Path<i32>,
    Query(query): Query<MessageQuery>,
) -> Result<Json<MessageListResponse>, AppError> {
    // Cek apakah user adalah participant
    let is_participant = state.conversation_repo
        .is_participant(conversation_id, participant.user_id)
        .await?;

    if !is_participant {
        return Err(AppError::forbidden("Tidak memiliki akses ke conversation ini"));
    }

    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.offset.unwrap_or(0);

    let messages = state.message_repo
        .get_conversation_messages(conversation_id, participant.user_id, limit, offset)
        .await?;

    // Ambil total message count
    let total = state.message_repo
        .get_conversation_message_count(conversation_id)
        .await?;

    tracing::info!("User {} mengambil {} messages dari conversation {}",
                   participant.user_id, messages.len(), conversation_id);

    Ok(Json(MessageListResponse {
        messages,
        total,
        limit,
        offset,
        conversation_id,
    }))
}

// Ambil detail message berdasarkan ID
#[utoipa::path(
    get,
    path = "/messages/{message_id}",
    tag = "messages",
    security(("bearer_auth" = [])),
    params(
        ("message_id" = i32, Path, description = "Message ID")
    ),
    responses(
        (status = 200, description = "Message berhasil diambil", body = Message),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Tidak memiliki akses"),
        (status = 404, description = "Message tidak ditemukan"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_message_by_id(
    State(state): State<AppState>,
    participant: ChatParticipant,
    Path(message_id): Path<i32>,
) -> Result<Json<Message>, AppError> {
    let message = state.message_repo
        .get_message_by_id(message_id, participant.user_id)
        .await?;

    match message {
        Some(msg) => {
            tracing::info!("User {} mengakses message {}", participant.user_id, message_id);
            Ok(Json(msg))
        }
        None => Err(AppError::not_found("Message tidak ditemukan")),
    }
}

// Tandai message sebagai sudah dibaca dengan broadcast update
#[utoipa::path(
    post,
    path = "/messages/{message_id}/read",
    tag = "messages",
    security(("bearer_auth" = [])),
    params(
        ("message_id" = i32, Path, description = "Message ID")
    ),
    responses(
        (status = 204, description = "Message ditandai sebagai sudah dibaca"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Tidak memiliki akses"),
        (status = 404, description = "Message tidak ditemukan"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn mark_message_read(
    State(state): State<AppState>,
    participant: ChatParticipant,
    Path(message_id): Path<i32>,
) -> Result<StatusCode, AppError> {
    // Cek apakah message ada dan user adalah participant
    let message = state.message_repo
        .get_message_by_id(message_id, participant.user_id)
        .await?;

    if message.is_none() {
        return Err(AppError::not_found("Message tidak ditemukan"));
    }

    let message = message.unwrap();

    // Tandai message sebagai sudah dibaca
    state.message_repo
        .mark_message_as_read(message_id, participant.user_id)
        .await?;

    // Broadcast read status update via NATS
    if let Some(nats_client) = &state.nats_client {
        let read_payload = serde_json::json!({
            "type": "message_read",
            "conversation_id": message.conversation_id,
            "message_id": message_id,
            "read_by": participant.user_id,
            "read_at": chrono::Utc::now()
        });

        let subject = format!("chat.{}", message.conversation_id);
        if let Err(e) = nats_client
            .publish(subject, read_payload.to_string().into())
            .await
        {
            tracing::warn!("Gagal broadcast read status: {}", e);
        }
    }

    tracing::info!("User {} menandai message {} sebagai sudah dibaca", participant.user_id, message_id);

    Ok(StatusCode::NO_CONTENT)
}

// Hapus message (hanya oleh sender) dengan broadcast notification
#[utoipa::path(
    delete,
    path = "/messages/{message_id}",
    tag = "messages",
    security(("bearer_auth" = [])),
    params(
        ("message_id" = i32, Path, description = "Message ID")
    ),
    responses(
        (status = 204, description = "Message berhasil dihapus"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Hanya sender yang bisa menghapus message"),
        (status = 404, description = "Message tidak ditemukan"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn delete_message(
    State(state): State<AppState>,
    participant: ChatParticipant,
    Path(message_id): Path<i32>,
) -> Result<StatusCode, AppError> {
    // Cek apakah message ada dan milik user
    let message = state.message_repo
        .get_message_by_id(message_id, participant.user_id)
        .await?;

    if message.is_none() {
        return Err(AppError::not_found("Message tidak ditemukan"));
    }

    // Verifikasi bahwa user adalah sender
    let message = message.unwrap();
    if message.sender_id != participant.user_id {
        return Err(AppError::forbidden("Hanya sender yang bisa menghapus message"));
    }

    let conversation_id = message.conversation_id;

    // Hapus message
    let deleted = state.message_repo
        .delete_message(message_id, participant.user_id)
        .await?;

    if deleted {
        // Broadcast message deletion via NATS
        if let Some(nats_client) = &state.nats_client {
            let delete_payload = serde_json::json!({
                "type": "message_deleted",
                "conversation_id": conversation_id,
                "message_id": message_id,
                "deleted_by": participant.user_id,
                "deleted_at": chrono::Utc::now()
            });

            let subject = format!("chat.{}", conversation_id);
            if let Err(e) = nats_client
                .publish(subject, delete_payload.to_string().into())
                .await
            {
                tracing::warn!("Gagal broadcast message deletion: {}", e);
            }
        }

        tracing::info!("User {} menghapus message {}", participant.user_id, message_id);
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::internal("Gagal menghapus message"))
    }
}

// Ambil latest message dalam conversation
#[utoipa::path(
    get,
    path = "/messages/latest/{conversation_id}",
    tag = "messages",
    security(("bearer_auth" = [])),
    params(
        ("conversation_id" = i32, Path, description = "Conversation ID")
    ),
    responses(
        (status = 200, description = "Latest message berhasil diambil", body = Option<Message>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Tidak memiliki akses ke conversation"),
        (status = 404, description = "Conversation tidak ditemukan"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_latest_message(
    State(state): State<AppState>,
    participant: ChatParticipant,
    Path(conversation_id): Path<i32>,
) -> Result<Json<Option<Message>>, AppError> {
    // Cek apakah user adalah participant
    let is_participant = state.conversation_repo
        .is_participant(conversation_id, participant.user_id)
        .await?;

    if !is_participant {
        return Err(AppError::forbidden("Tidak memiliki akses ke conversation ini"));
    }

    let message = state.message_repo
        .get_latest_message(conversation_id)
        .await?;

    tracing::info!("User {} mengambil latest message dari conversation {}",
                   participant.user_id, conversation_id);

    Ok(Json(message))
}

// Ambil jumlah messages dalam conversation
#[utoipa::path(
    get,
    path = "/messages/count/{conversation_id}",
    tag = "messages",
    security(("bearer_auth" = [])),
    params(
        ("conversation_id" = i32, Path, description = "Conversation ID")
    ),
    responses(
        (status = 200, description = "Message count berhasil diambil", body = MessageCountResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Tidak memiliki akses ke conversation"),
        (status = 404, description = "Conversation tidak ditemukan"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_message_count(
    State(state): State<AppState>,
    participant: ChatParticipant,
    Path(conversation_id): Path<i32>,
) -> Result<Json<MessageCountResponse>, AppError> {
    // Cek apakah user adalah participant
    let is_participant = state.conversation_repo
        .is_participant(conversation_id, participant.user_id)
        .await?;

    if !is_participant {
        return Err(AppError::forbidden("Tidak memiliki akses ke conversation ini"));
    }

    // Ambil total messages
    let total_messages = state.message_repo
        .get_conversation_message_count(conversation_id)
        .await?;

    // Ambil unread messages untuk user
    let unread_messages = state.message_repo
        .get_conversation_unread_count(conversation_id, participant.user_id)
        .await?;

    tracing::info!("User {} mengambil count dari conversation {}: {} total, {} unread",
                   participant.user_id, conversation_id, total_messages, unread_messages);

    Ok(Json(MessageCountResponse {
        total_messages,
        unread_messages,
        conversation_id,
    }))
}

// Ambil jumlah unread messages dalam conversation
#[utoipa::path(
    get,
    path = "/messages/unread/{conversation_id}",
    tag = "messages",
    security(("bearer_auth" = [])),
    params(
        ("conversation_id" = i32, Path, description = "Conversation ID")
    ),
    responses(
        (status = 200, description = "Jumlah unread messages berhasil diambil", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Tidak memiliki akses"),
        (status = 404, description = "Conversation tidak ditemukan"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_unread_count(
    State(state): State<AppState>,
    participant: ChatParticipant,
    Path(conversation_id): Path<i32>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Cek apakah user adalah participant
    let is_participant = state.conversation_repo
        .is_participant(conversation_id, participant.user_id)
        .await?;

    if !is_participant {
        return Err(AppError::forbidden("Tidak memiliki akses ke conversation ini"));
    }

    let unread_count = state.message_repo
        .get_conversation_unread_count(conversation_id, participant.user_id)
        .await?;

    tracing::info!("User {} memiliki {} unread messages dalam conversation {}",
                   participant.user_id, unread_count, conversation_id);

    Ok(Json(serde_json::json!({
        "unread_count": unread_count,
        "conversation_id": conversation_id,
        "user_id": participant.user_id
    })))
}

// Search messages dalam conversation
#[utoipa::path(
    get,
    path = "/messages/search",
    tag = "messages",
    security(("bearer_auth" = [])),
    params(
        ("conversation_id" = Option<i32>, Query, description = "Filter by conversation ID"),
        ("search" = String, Query, description = "Search query string"),
        ("limit" = Option<i64>, Query, description = "Limit results (max 50)"),
        ("offset" = Option<i64>, Query, description = "Offset for pagination")
    ),
    responses(
        (status = 200, description = "Search results berhasil diambil", body = MessageListResponse),
        (status = 400, description = "Query search diperlukan atau tidak valid"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Tidak memiliki akses ke conversation"),
        (status = 404, description = "Conversation tidak ditemukan"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn search_messages(
    State(state): State<AppState>,
    participant: ChatParticipant,
    Query(query): Query<MessageQuery>,
) -> Result<Json<MessageListResponse>, AppError> {
    // Get conversation_id from query params
    let conversation_id = query.conversation_id.ok_or_else(|| {
        AppError::bad_request("Conversation ID diperlukan")
    })?;

    // Cek apakah user adalah participant
    let is_participant = state.conversation_repo
        .is_participant(conversation_id, participant.user_id)
        .await?;

    if !is_participant {
        return Err(AppError::forbidden("Tidak memiliki akses ke conversation ini"));
    }

    let search_query = query.search.ok_or_else(|| {
        AppError::bad_request("Query search diperlukan")
    })?;

    if search_query.trim().is_empty() {
        return Err(AppError::bad_request("Query search tidak boleh kosong"));
    }

    let limit = query.limit.unwrap_or(20).min(50);
    let offset = query.offset.unwrap_or(0);

    let messages = state.message_repo
        .search_conversation_messages(conversation_id, participant.user_id, &search_query, limit, offset)
        .await?;

    let total = messages.len() as i64;

    tracing::info!("User {} search '{}' dalam conversation {} menemukan {} results",
                   participant.user_id, search_query, conversation_id, total);

    Ok(Json(MessageListResponse {
        messages,
        total,
        limit,
        offset,
        conversation_id,
    }))
}

// Ambil media messages (images, files) dalam conversation
#[utoipa::path(
    get,
    path = "/messages/media/{conversation_id}",
    tag = "messages",
    security(("bearer_auth" = [])),
    params(
        ("conversation_id" = i32, Path, description = "Conversation ID"),
        MessageQuery
    ),
    responses(
        (status = 200, description = "Media messages berhasil diambil", body = MessageListResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Tidak memiliki akses"),
        (status = 404, description = "Conversation tidak ditemukan"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_media_messages(
    State(state): State<AppState>,
    participant: ChatParticipant,
    Path(conversation_id): Path<i32>,
    Query(query): Query<MessageQuery>,
) -> Result<Json<MessageListResponse>, AppError> {
    // Cek apakah user adalah participant
    let is_participant = state.conversation_repo
        .is_participant(conversation_id, participant.user_id)
        .await?;

    if !is_participant {
        return Err(AppError::forbidden("Tidak memiliki akses ke conversation ini"));
    }

    let limit = query.limit.unwrap_or(20).min(50);
    let offset = query.offset.unwrap_or(0);

    let messages = state.message_repo
        .get_media_messages(conversation_id, participant.user_id, limit, offset)
        .await?;

    let total = messages.len() as i64;

    tracing::info!("User {} mengambil {} media messages dari conversation {}",
                   participant.user_id, total, conversation_id);

    Ok(Json(MessageListResponse {
        messages,
        total,
        limit,
        offset,
        conversation_id,
    }))
}

// Ambil messages dari sender tertentu dalam conversation
#[utoipa::path(
    get,
    path = "/conversations/{conversation_id}/messages/sender/{sender_id}",
    tag = "messages",
    security(("bearer_auth" = [])),
    params(
        ("conversation_id" = i32, Path, description = "Conversation ID"),
        ("sender_id" = i32, Path, description = "Sender ID"),
        MessageQuery
    ),
    responses(
        (status = 200, description = "Messages dari sender berhasil diambil", body = MessageListResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Tidak memiliki akses"),
        (status = 404, description = "Conversation tidak ditemukan"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_messages_by_sender(
    State(state): State<AppState>,
    participant: ChatParticipant,
    Path((conversation_id, sender_id)): Path<(i32, i32)>,
    Query(query): Query<MessageQuery>,
) -> Result<Json<MessageListResponse>, AppError> {
    // Cek apakah user adalah participant
    let is_participant = state.conversation_repo
        .is_participant(conversation_id, participant.user_id)
        .await?;

    if !is_participant {
        return Err(AppError::forbidden("Tidak memiliki akses ke conversation ini"));
    }

    let limit = query.limit.unwrap_or(20).min(50);
    let offset = query.offset.unwrap_or(0);

    let messages = state.message_repo
        .get_messages_by_sender(conversation_id, sender_id, limit, offset)
        .await?;

    let total = messages.len() as i64;

    tracing::info!("User {} mengambil {} messages dari sender {} dalam conversation {}",
                   participant.user_id, total, sender_id, conversation_id);

    Ok(Json(MessageListResponse {
        messages,
        total,
        limit,
        offset,
        conversation_id,
    }))
}

// Request struct untuk message dengan files
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateMessageWithFilesRequest {
    pub content: String,
    pub message_type: MessageType,
    pub files: Option<Vec<String>>,            
    pub thumbnails: Option<Vec<String>>,       
}

// Kirim message dengan files (terintegrasi dengan upload handler)
#[utoipa::path(
    post,
    path = "/messages/with-files",
    tag = "messages",
    security(("bearer_auth" = [])),
    request_body = CreateMessageWithFilesRequest,
    responses(
        (status = 201, description = "Message dengan files berhasil dikirim", body = Message),
        (status = 400, description = "Request tidak valid"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Tidak memiliki akses ke conversation"),
        (status = 404, description = "Conversation tidak ditemukan"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn send_message_with_files(
    State(state): State<AppState>,
    participant: ChatParticipant,
    Path(conversation_id): Path<i32>,
    Json(request): Json<CreateMessageWithFilesRequest>,
) -> Result<Json<Message>, AppError> {
    // Cek apakah user adalah participant dalam conversation
    let is_participant = state.conversation_repo
        .is_participant(conversation_id, participant.user_id)
        .await?;

    if !is_participant {
        return Err(AppError::forbidden("Tidak memiliki akses ke conversation ini"));
    }

    // Validate files jika ada
    if let Some(ref files) = request.files {
        validate_chat_files(files)?;
    }

    // Extract file info untuk message creation menggunakan utility function
    let upload_response = UploadResponse {
        success: true,
        files: request.files.as_ref().map_or(vec![], |files| {
            files.iter().enumerate().map(|(i, file_url)| UploadedFile {
                filename: format!("file-{}", i),
                original_name: Some(format!("file-{}", i)),
                file_type: "unknown".to_string(),
                file_size: 0,
                url: file_url.clone(),
                thumbnail_url: request.thumbnails.as_ref()
                    .and_then(|thumbs| thumbs.get(i))
                    .cloned(),
                category: FileCategory::Image, 
            }).collect()
        }),
        message: "Files processed".to_string(),
    };

    let file_info = extract_file_info_for_message(&upload_response);
    let (media_url, thumbnail_url) = if !file_info.is_empty() {
        (Some(file_info[0].0.clone()), file_info[0].1.clone())
    } else {
        (None, None)
    };

    // Generate message request untuk database
    let create_request = CreateMessageRequest {
        conversation_id,
        content: request.content,
        message_type: Some(request.message_type.as_str().to_string()),
        media_url,
        thumbnail_url,
    };

    // Buat message baru
    let message = state.message_repo
        .create_message(conversation_id, participant.user_id, create_request)
        .await?;

    // Update last message info di conversation
    let content_preview = if message.content.len() > 50 {
        format!("{}...", &message.content[..50])
    } else {
        message.content.clone()
    };

    state.conversation_repo
        .update_last_message(conversation_id, &content_preview)
        .await?;

    // Broadcast message via NATS ke WebSocket connections
    if let Some(nats_client) = &state.nats_client {
        if let Err(e) = broadcast_message_via_nats(
            nats_client,
            conversation_id,
            &message,
            &participant.email,
        ).await {
            // Log error tapi tidak gagalkan request, karena message sudah tersimpan di database
            tracing::warn!("Broadcast message gagal: {}, tapi message tersimpan di database", e);
        }
    } else {
        tracing::warn!("NATS client tidak tersedia, message tidak di-broadcast secara real-time");
    }

    tracing::info!("User {} mengirim message {} dengan files ke conversation {}",
                   participant.user_id, message.id, conversation_id);

    Ok(Json(message))
}

// Typing indicator request
#[derive(Debug, Deserialize, ToSchema)]
pub struct TypingIndicatorRequest {
    pub conversation_id: i32,
    pub is_typing: bool,
}

// Broadcast typing indicator ke conversation
#[utoipa::path(
    post,
    path = "/messages/typing",
    tag = "messages",
    security(("bearer_auth" = [])),
    request_body = TypingIndicatorRequest,
    responses(
        (status = 200, description = "Typing indicator broadcasted"),
        (status = 400, description = "Request tidak valid"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Tidak memiliki akses"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn send_typing_indicator(
    State(state): State<AppState>,
    participant: ChatParticipant,
    Path(conversation_id): Path<i32>,
    Json(request): Json<TypingIndicatorRequest>,
) -> Result<StatusCode, AppError> {
    // Validasi role participant
    if !participant.is_customer() && !participant.is_seller() {
        return Err(AppError::forbidden("Role tidak valid untuk typing indicator"));
    }

    // Validate conversation ID matches path parameter
    if request.conversation_id != conversation_id {
        return Err(AppError::bad_request("Conversation ID mismatch"));
    }

    // Cek apakah user adalah participant
    let is_participant = state.conversation_repo
        .is_participant(conversation_id, participant.user_id)
        .await?;

    if !is_participant {
        return Err(AppError::forbidden("Tidak memiliki akses ke conversation ini"));
    }

    // Broadcast typing indicator via NATS
    if let Some(nats_client) = &state.nats_client {
        let typing_payload = serde_json::json!({
            "type": "typing_indicator",
            "conversation_id": conversation_id,
            "user_id": participant.user_id,
            "user_email": participant.email,
            "is_typing": request.is_typing,
            "timestamp": chrono::Utc::now()
        });

        let subject = format!("chat.{}", conversation_id);
        if let Err(e) = nats_client
            .publish(subject, typing_payload.to_string().into())
            .await
        {
            tracing::warn!("Gagal broadcast typing indicator: {}", e);
        } else {
            tracing::info!("User {} {} di conversation {}",
                          participant.user_id,
                          if request.is_typing { "sedang mengetik" } else { "berhenti mengetik" },
                          conversation_id);
        }
    } else {
        tracing::warn!("NATS client tidak tersedia, typing indicator tidak di-broadcast");
    }

    Ok(StatusCode::OK)
}

// Message preview untuk files
#[derive(Debug, Serialize, ToSchema)]
pub struct MessagePreviewResponse {
    pub preview_text: String,
    pub file_count: usize,
    pub has_images: bool,
    pub has_documents: bool,
}

// Generate preview text untuk message dengan files
#[utoipa::path(
    post,
    path = "/messages/preview",
    tag = "messages",
    security(("bearer_auth" = [])),
    request_body = UploadResponse,
    responses(
        (status = 200, description = "Message preview berhasil dibuat", body = MessagePreviewResponse),
        (status = 400, description = "Request tidak valid"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn generate_message_preview(
    State(_state): State<AppState>,
    participant: ChatParticipant,
    Json(request): Json<UploadResponse>,
) -> Result<Json<MessagePreviewResponse>, AppError> {
    tracing::info!("User {} generates message preview with {} files",
                   participant.user_id, request.files.len());

    let preview_text = generate_preview_text(&request.files);
    let image_count = request.files.iter()
        .filter(|f| matches!(f.category, FileCategory::Image))
        .count();
    let doc_count = request.files.iter()
        .filter(|f| matches!(f.category, FileCategory::Document))
        .count();

    Ok(Json(MessagePreviewResponse {
        preview_text,
        file_count: request.files.len(),
        has_images: image_count > 0,
        has_documents: doc_count > 0,
    }))
}