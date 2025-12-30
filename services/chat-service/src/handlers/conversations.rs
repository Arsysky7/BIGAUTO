// Conversation Handlers untuk Chat Service
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    config::AppState,
    domain::conversation::{CreateConversationRequest, ConversationResponse},
    middleware::{ChatParticipant, AuthUser},
    error::AppError,
};

// Query parameters untuk pagination
#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct PaginationQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// Response untuk conversation list
#[derive(Debug, Serialize, ToSchema)]
pub struct ConversationListResponse {
    pub conversations: Vec<ConversationResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

// Response untuk conversation dengan details
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ConversationWithDetailsResponse {
    pub id: i32,
    pub customer_id: i32,
    pub seller_id: i32,
    pub seller_name: String,
    pub vehicle_id: Option<i32>,
    pub vehicle_title: Option<String>,
    pub last_message: Option<String>,
    pub last_message_at: Option<chrono::DateTime<chrono::Utc>>,
    pub unread_count: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Default for PaginationQuery {
    fn default() -> Self {
        Self {
            limit: Some(20),
            offset: Some(0),
        }
    }
}

// Buat conversation baru
#[utoipa::path(
    post,
    path = "/conversations",
    tag = "conversations",
    security(("bearer_auth" = [])),
    request_body = CreateConversationRequest,
    responses(
        (status = 201, description = "Conversation berhasil dibuat", body = ConversationResponse),
        (status = 400, description = "Request tidak valid"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn create_conversation(
    State(state): State<AppState>,
    user: AuthUser,
    Json(request): Json<CreateConversationRequest>,
) -> Result<(StatusCode, Json<ConversationResponse>), AppError> {
    // Validasi role user - hanya customer yang bisa membuat conversation
    if !user.is_customer() {
        return Err(AppError::forbidden("Hanya customer yang bisa membuat conversation baru"));
    }

    // Validasi request
    if request.seller_id == user.user_id {
        return Err(AppError::bad_request("Tidak bisa membuat conversation dengan diri sendiri"));
    }

    // Cek apakah conversation sudah ada antara user ini dan seller dengan vehicle yang sama
    let existing_conversation = sqlx::query!(
        r#"
        SELECT c.id, c.customer_id, c.seller_id, c.vehicle_id,
               c.last_message, c.last_message_at, c.created_at, c.updated_at,
               u.name as seller_name, v.title as vehicle_title
        FROM conversations c
        JOIN users u ON c.seller_id = u.id
        LEFT JOIN vehicles v ON c.vehicle_id = v.id
        WHERE (c.customer_id = $1 AND c.seller_id = $2 AND c.vehicle_id = $3)
           OR (c.customer_id = $3 AND c.seller_id = $2 AND c.vehicle_id = $1)
        ORDER BY c.created_at DESC
        LIMIT 1
        "#,
        user.user_id, request.seller_id, request.vehicle_id
    )
    .fetch_optional(&state.db)
    .await?;

    if let Some(conv) = existing_conversation {
        // Hitung unread count yang REAL dari database
        let unread_count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM messages
             WHERE conversation_id = $1 AND sender_id != $2 AND is_read = false",
            conv.id,
            user.user_id
        )
        .fetch_one(&state.db)
        .await?
        .unwrap_or(0);

        let response = ConversationResponse {
            id: conv.id,
            customer_id: conv.customer_id,
            seller_id: conv.seller_id,
            seller_name: conv.seller_name,
            vehicle_id: conv.vehicle_id,
            vehicle_title: Some(conv.vehicle_title),
            last_message: conv.last_message,
            last_message_at: conv.last_message_at,
            unread_count,
            created_at: conv.created_at.unwrap_or_else(|| chrono::Utc::now()),
            updated_at: conv.updated_at.unwrap_or_else(|| chrono::Utc::now()),
        };

        tracing::info!("Existing conversation {} found for user {} with {} unread messages",
                      conv.id, user.user_id, unread_count);

        return Ok((StatusCode::OK, Json(response)));
    }

    // Buat conversation baru
    let conversation_id = sqlx::query_scalar!(
        r#"
        INSERT INTO conversations (customer_id, seller_id, vehicle_id, created_at, updated_at)
        VALUES ($1, $2, $3, NOW(), NOW())
        RETURNING id
        "#,
        user.user_id, request.seller_id, request.vehicle_id
    )
    .fetch_one(&state.db)
    .await?;

    // Ambil conversation yang baru dibuat dengan details
    let conversation = sqlx::query!(
        r#"
        SELECT c.id, c.customer_id, c.seller_id, c.vehicle_id,
               c.last_message, c.last_message_at, c.created_at, c.updated_at,
               u.name as seller_name, v.title as vehicle_title
        FROM conversations c
        JOIN users u ON c.seller_id = u.id
        LEFT JOIN vehicles v ON c.vehicle_id = v.id
        WHERE c.id = $1
        "#,
        conversation_id
    )
    .fetch_one(&state.db)
    .await?;

    let response = ConversationResponse {
        id: conversation.id,
        customer_id: conversation.customer_id,
        seller_id: conversation.seller_id,
        seller_name: conversation.seller_name,
        vehicle_id: conversation.vehicle_id,
        vehicle_title: Some(conversation.vehicle_title),
        last_message: conversation.last_message,
        last_message_at: conversation.last_message_at,
        unread_count: 0, 
        created_at: conversation.created_at.unwrap_or_else(|| chrono::Utc::now()),
        updated_at: conversation.updated_at.unwrap_or_else(|| chrono::Utc::now()),
    };

    tracing::info!("Conversation {} created by user {} with seller {} for vehicle {}",
                  conversation_id, user.user_id, request.seller_id,
                  request.vehicle_id.unwrap_or(0));

    Ok((StatusCode::CREATED, Json(response)))
}

// Ambil conversations user (customer atau seller)
#[utoipa::path(
    get,
    path = "/conversations/user/{user_id}",
    tag = "conversations",
    security(("bearer_auth" = [])),
    params(
        ("user_id" = i32, Path, description = "User ID"),
        PaginationQuery
    ),
    responses(
        (status = 200, description = "Daftar conversations berhasil diambil", body = ConversationListResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_user_conversations(
    State(state): State<AppState>,
    participant: ChatParticipant,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<ConversationListResponse>, AppError> {
    let limit = query.limit.unwrap_or(20).min(100);
    let offset = query.offset.unwrap_or(0);

    // Query conversations dengan join ke users dan vehicles untuk response lengkap
    let conversations_raw = sqlx::query!(
        r#"
        SELECT c.id, c.customer_id, c.seller_id, c.vehicle_id,
               c.last_message, c.last_message_at, c.created_at, c.updated_at,
               cu.name as customer_name,
               su.name as seller_name,
               v.title as vehicle_title
        FROM conversations c
        JOIN users cu ON c.customer_id = cu.id
        JOIN users su ON c.seller_id = su.id
        LEFT JOIN vehicles v ON c.vehicle_id = v.id
        WHERE c.customer_id = $1 OR c.seller_id = $1
        ORDER BY c.updated_at DESC
        LIMIT $2 OFFSET $3
        "#,
        participant.user_id, limit, offset
    )
    .fetch_all(&state.db)
    .await?;

    // Build response dengan unread count untuk setiap conversation
    let mut conversations = Vec::new();
    for conv in conversations_raw {
        let unread_count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM messages
             WHERE conversation_id = $1 AND sender_id != $2 AND is_read = false",
            conv.id,
            participant.user_id
        )
        .fetch_one(&state.db)
        .await?
        .unwrap_or(0);

        let response = ConversationResponse {
            id: conv.id,
            customer_id: conv.customer_id,
            seller_id: conv.seller_id,
            seller_name: conv.seller_name,
            vehicle_id: conv.vehicle_id,
            vehicle_title: Some(conv.vehicle_title),
            last_message: conv.last_message,
            last_message_at: conv.last_message_at,
            unread_count,
            created_at: conv.created_at.unwrap_or_else(|| chrono::Utc::now()),
            updated_at: conv.updated_at.unwrap_or_else(|| chrono::Utc::now()),
        };
        conversations.push(response);
    }

    // Hitung total conversations untuk user ini
    let total = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM conversations
         WHERE customer_id = $1 OR seller_id = $1",
        participant.user_id
    )
    .fetch_one(&state.db)
    .await?
    .unwrap_or(0);

    tracing::info!("User {} retrieved {} conversations ({} total)",
                  participant.user_id, conversations.len(), total);

    Ok(Json(ConversationListResponse {
        conversations,
        total,
        limit,
        offset,
    }))
}

// Ambil conversation berdasarkan ID
#[utoipa::path(
    get,
    path = "/conversations/{conversation_id}",
    tag = "conversations",
    security(("bearer_auth" = [])),
    params(
        ("conversation_id" = i32, Path, description = "Conversation ID")
    ),
    responses(
        (status = 200, description = "Conversation berhasil diambil", body = ConversationResponse),
        (status = 404, description = "Conversation tidak ditemukan"),
        (status = 403, description = "Tidak memiliki akses"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_conversation_by_id(
    State(state): State<AppState>,
    user: AuthUser,
    Path(conversation_id): Path<i32>,
) -> Result<Json<ConversationResponse>, AppError> {
    // Cek apakah conversation ada
    let conversation = sqlx::query!(
        r#"
        SELECT c.id, c.customer_id, c.seller_id, c.vehicle_id,
               c.last_message, c.last_message_at, c.created_at, c.updated_at,
               u.name as seller_name, v.title as vehicle_title
        FROM conversations c
        JOIN users u ON c.seller_id = u.id
        LEFT JOIN vehicles v ON c.vehicle_id = v.id
        WHERE c.id = $1
        "#,
        conversation_id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::not_found("Conversation tidak ditemukan"))?;

    // Gunakan AuthUser method untuk validasi akses
    if !user.can_access_conversation(conversation.customer_id, conversation.seller_id) {
        return Err(AppError::forbidden("Tidak memiliki akses ke conversation ini"));
    }

    // Hitung unread count
    let unread_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM messages
         WHERE conversation_id = $1 AND sender_id != $2 AND is_read = false",
        conversation_id,
        user.user_id
    )
    .fetch_one(&state.db)
    .await?
    .unwrap_or(0);

    let response = ConversationResponse {
        id: conversation.id,
        customer_id: conversation.customer_id,
        seller_id: conversation.seller_id,
        seller_name: conversation.seller_name,
        vehicle_id: conversation.vehicle_id,
        vehicle_title: Some(conversation.vehicle_title),
        last_message: conversation.last_message,
        last_message_at: conversation.last_message_at,
        unread_count,
        created_at: conversation.created_at.unwrap_or_else(|| chrono::Utc::now()),
        updated_at: conversation.updated_at.unwrap_or_else(|| chrono::Utc::now()),
    };

    // Gunakan AuthUser method untuk mendapatkan role dalam conversation
    let user_role = user.get_conversation_role(conversation.customer_id);
    tracing::info!("User {} (as {}) accessed conversation {} with {} unread messages",
                  user.user_id, user_role, conversation_id, unread_count);

    Ok(Json(response))
}

// Ambil conversation dengan details (info user, vehicle, unread count)
#[utoipa::path(
    get,
    path = "/conversations/{conversation_id}/details",
    tag = "conversations",
    security(("bearer_auth" = [])),
    params(
        ("conversation_id" = i32, Path, description = "Conversation ID")
    ),
    responses(
        (status = 200, description = "Conversation details berhasil diambil", body = ConversationWithDetailsResponse),
        (status = 404, description = "Conversation tidak ditemukan"),
        (status = 403, description = "Tidak memiliki akses"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_conversation_with_details(
    State(state): State<AppState>,
    participant: ChatParticipant,
    Path(conversation_id): Path<i32>,
) -> Result<Json<ConversationWithDetailsResponse>, AppError> {
    // Cek apakah participant memiliki role yang valid
    if !participant.role.contains("customer") && !participant.role.contains("seller") {
        return Err(AppError::forbidden("Invalid role for chat access"));
    }

    // Check if participant is active
    if !participant.is_active {
        return Err(AppError::forbidden("Account is not active"));
    }

    // Query dengan join ke customer dan seller untuk details lengkap
    let conversation = sqlx::query!(
        r#"
        SELECT c.id, c.customer_id, c.seller_id, c.vehicle_id,
               c.last_message, c.last_message_at, c.created_at, c.updated_at,
               cu.name as customer_name,
               su.name as seller_name,
               v.title as vehicle_title
        FROM conversations c
        JOIN users cu ON c.customer_id = cu.id
        JOIN users su ON c.seller_id = su.id
        LEFT JOIN vehicles v ON c.vehicle_id = v.id
        WHERE c.id = $1
        "#,
        conversation_id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::not_found("Conversation tidak ditemukan"))?;

    // Cek apakah user adalah participant
    if conversation.customer_id != participant.user_id && conversation.seller_id != participant.user_id {
        return Err(AppError::forbidden("Tidak memiliki akses ke conversation ini"));
    }

    // Hitung unread count untuk user ini
    let unread_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM messages
         WHERE conversation_id = $1 AND sender_id != $2 AND is_read = false",
        conversation_id,
        participant.user_id
    )
    .fetch_one(&state.db)
    .await?
    .unwrap_or(0);

    // Buat Conversation object dari query result
    let conversation_obj = crate::domain::Conversation {
        id: conversation.id,
        customer_id: conversation.customer_id,
        seller_id: conversation.seller_id,
        vehicle_id: conversation.vehicle_id,
        last_message: conversation.last_message,
        last_message_at: conversation.last_message_at,
        created_at: conversation.created_at.unwrap_or_else(|| chrono::Utc::now()),
        updated_at: conversation.updated_at.unwrap_or_else(|| chrono::Utc::now()),
    };

    // Buat ConversationWithDetails object
    let conversation_with_details = crate::domain::ConversationWithDetails::from_query_data(
        conversation_obj,
        conversation.customer_name,
        conversation.seller_name,
        Some(conversation.vehicle_title),
        unread_count,
    );

    // Convert ke response format
    let response_data = conversation_with_details.to_response_map();
    let response: ConversationWithDetailsResponse = serde_json::from_value(response_data)
        .map_err(|e| {
            tracing::error!("Failed to convert ConversationWithDetails to response: {}", e);
            AppError::internal("Failed to process conversation data")
        })?;

    tracing::info!("User {} accessed detailed conversation {} with {} unread messages",
                  participant.user_id, conversation_id, unread_count);

    Ok(Json(response))
}

// Tandai conversation sebagai sudah dibaca
#[utoipa::path(
    post,
    path = "/conversations/{conversation_id}/read",
    tag = "conversations",
    security(("bearer_auth" = [])),
    params(
        ("conversation_id" = i32, Path, description = "Conversation ID")
    ),
    responses(
        (status = 204, description = "Conversation ditandai sebagai sudah dibaca"),
        (status = 404, description = "Conversation tidak ditemukan"),
        (status = 403, description = "Tidak memiliki akses"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn mark_conversation_read(
    State(state): State<AppState>,
    participant: ChatParticipant,
    Path(conversation_id): Path<i32>,
) -> Result<StatusCode, AppError> {
    // Cek apakah conversation ada dan user adalah participant
    let conversation = sqlx::query!(
        "SELECT customer_id, seller_id FROM conversations WHERE id = $1",
        conversation_id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::not_found("Conversation tidak ditemukan"))?;

    // Cek apakah user adalah participant
    if conversation.customer_id != participant.user_id && conversation.seller_id != participant.user_id {
        return Err(AppError::forbidden("Tidak memiliki akses ke conversation ini"));
    }

    // Update semua messages yang belum dibaca dari user lain
    let updated_rows = sqlx::query!(
        r#"
        UPDATE messages
        SET is_read = true, read_at = NOW()
        WHERE conversation_id = $1 AND sender_id != $2 AND is_read = false
        "#,
        conversation_id, participant.user_id
    )
    .execute(&state.db)
    .await?
    .rows_affected();

    tracing::info!("User {} marked {} messages as read in conversation {}",
                  participant.user_id, updated_rows, conversation_id);

    Ok(StatusCode::NO_CONTENT)
}

// Ambil jumlah unread messages untuk conversation
#[utoipa::path(
    get,
    path = "/conversations/unread",
    tag = "conversations",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Total unread messages untuk user (semua conversations)", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_unread_count(
    State(state): State<AppState>,
    participant: ChatParticipant,
) -> Result<Json<serde_json::Value>, AppError> {
    // Hitung total unread messages untuk user ini
    let unread_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM messages m
         JOIN conversations c ON m.conversation_id = c.id
         WHERE (c.customer_id = $1 OR c.seller_id = $1)
           AND m.sender_id != $1
           AND m.is_read = false",
        participant.user_id
    )
    .fetch_one(&state.db)
    .await?
    .unwrap_or(0);

    tracing::info!("User {} has {} unread messages", participant.user_id, unread_count);

    Ok(Json(serde_json::json!({
        "user_id": participant.user_id,
        "unread_count": unread_count,
        "calculated_at": chrono::Utc::now().to_rfc3339()
    })))
}

// Health check endpoint
#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses(
        (status = 200, description = "Service health check", body = crate::config::HealthCheckResponse)
    )
)]
pub async fn health_check(
    State(state): State<AppState>,
) -> Json<crate::config::HealthCheckResponse> {
    Json(state.health_check().await)
}