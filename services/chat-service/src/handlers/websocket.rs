// WebSocket Handler untuk Real-time Chat
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State, Path,
    },
    response::Response,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use uuid::Uuid;
use async_nats::Client;

use crate::{
    config::AppState,
    middleware::WebSocketParticipant,
    error::AppError,
    domain::message::TypingIndicator,
};

// WebSocket message types
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMessage {
    // Client messages
    Ping,
    Subscribe { conversation_id: i32 },
    Unsubscribe { conversation_id: i32 },
    TypingStart { conversation_id: i32 },
    TypingStop { conversation_id: i32 },

    // Server messages
    Pong,
    Subscribed { conversation_id: i32 },
    Unsubscribed { conversation_id: i32 },
    NewMessage {
        conversation_id: i32,
        message: serde_json::Value,
    },
    MessageRead {
        conversation_id: i32,
        message_id: i32,
        read_by: i32,
    },
    MessageDeleted {
        conversation_id: i32,
        message_id: i32,
        deleted_by: i32,
    },
    UserTyping {
        conversation_id: i32,
        user_id: i32,
        user_email: String,
        is_typing: bool,
    },
    Error {
        code: String,
        message: String,
    },
}

// WebSocket connection info
#[derive(Debug, Clone)]
pub struct WsConnection {
    pub user_id: i32,
    pub user_email: String,
    pub user_role: String,
    pub conversation_subscriptions: Arc<RwLock<HashMap<i32, bool>>>,
    pub is_alive: Arc<RwLock<bool>>,
}

// Active connections manager - Manajer koneksi WebSocket aktif
pub struct ConnectionManager {
    connections: Arc<RwLock<HashMap<Uuid, Arc<WsConnection>>>>,
}

// Global connection manager instance untuk tracking semua koneksi aktif
lazy_static::lazy_static! {
    static ref CONNECTION_MANAGER: ConnectionManager = ConnectionManager::new();
}

impl ConnectionManager {
    // Buat instance baru
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // Tambahkan koneksi baru ke manager
    pub async fn tambah_koneksi(connection_id: Uuid, connection: Arc<WsConnection>) {
        let mut manager = CONNECTION_MANAGER.connections.write().await;
        manager.insert(connection_id, connection.clone());
        tracing::info!("Koneksi {} ditambahkan ke manager. Total koneksi: {}",
                      connection_id, manager.len());
    }

    // Hapus koneksi dari manager
    pub async fn hapus_koneksi(connection_id: &Uuid) {
        let mut manager = CONNECTION_MANAGER.connections.write().await;
        manager.remove(connection_id);
        tracing::info!("Koneksi {} dihapus dari manager. Total koneksi: {}",
                      connection_id, manager.len());
    }

  
  
    // Ambil total jumlah koneksi aktif
    pub async fn total_koneksi() -> usize {
        let manager = CONNECTION_MANAGER.connections.read().await;
        manager.len()
    }
}

// Process NATS messages dan forward ke WebSocket
async fn process_nats_messages(
    nats_client: &Client,
    connection_id: Uuid,
    connection: Arc<WsConnection>,
    tx: Arc<Mutex<futures::stream::SplitSink<WebSocket, Message>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Subscribe ke user-specific messages
    let user_sub = nats_client
        .subscribe(format!("chat.{}", connection.user_id))
        .await?;

    tracing::info!("Connection {} subscribed ke NATS user topic", connection_id);

    // Subscribe ke semua conversation subscriptions
    let subscriptions = connection.conversation_subscriptions.read().await;
    let mut conversation_subs = Vec::new();

    for (&conv_id, _) in subscriptions.iter() {
        let sub = nats_client
            .subscribe(format!("chat.{}", conv_id))
            .await?;
        conversation_subs.push((conv_id, sub));
    }
    drop(subscriptions);

    // Process NATS messages
    let mut user_messages = user_sub;
    let tx_clone = tx.clone();

    // User-specific messages handler
    tokio::spawn(async move {
        while let Some(nats_msg) = user_messages.next().await {
            if let Ok(text) = String::from_utf8(nats_msg.payload.into()) {
                if let Ok(ws_message) = serde_json::from_str::<serde_json::Value>(&text) {
                    if let Ok(ws_text) = serde_json::to_string(&ws_message) {
                        let mut tx_lock = tx_clone.lock().await;
                        if tx_lock.send(Message::Text(ws_text.into())).await.is_err() {
                            break;
                        }
                    }
                }
            }
        }
    });

    // Conversation-specific messages handler
    for (conv_id, sub) in conversation_subs {
        let tx_conv = tx.clone();
        let connection = connection.clone();

        // Log subscription untuk debugging
        tracing::debug!("Connection {} subscribed ke conversation {} via NATS", connection_id, conv_id);

        tokio::spawn(async move {
            let mut conv_messages = sub;
            while let Some(nats_msg) = conv_messages.next().await {
                if let Ok(text) = String::from_utf8(nats_msg.payload.into()) {
                    // Check jika ini adalah TypingIndicator
                    if let Ok(typing_indicator) = serde_json::from_str::<crate::domain::message::TypingIndicator>(&text) {
                        // Filter typing indicator yang tidak dari user ini sendiri
                        if typing_indicator.user_id == connection.user_id {
                            continue;
                        }

                        // Convert ke WebSocket message dan kirim
                        let ws_message = typing_indicator.to_websocket_message();
                        if let Ok(ws_text) = serde_json::to_string(&ws_message) {
                            let mut tx_lock = tx_conv.lock().await;
                            if tx_lock.send(Message::Text(ws_text.into())).await.is_err() {
                                break;
                            }

                            tracing::debug!("Typing indicator sent to connection {} for conversation {}",
                                           connection_id, conv_id);
                        }
                    } else if let Ok(ws_message) = serde_json::from_str::<serde_json::Value>(&text) {
                        // Filter messages yang tidak dari user ini sendiri
                        if let Some(sender_id) = ws_message.get("sender_id") {
                            if let Some(sender_num) = sender_id.as_i64() {
                                if sender_num == connection.user_id as i64 {
                                    continue;
                                }
                            }
                        }

                        if let Ok(ws_text) = serde_json::to_string(&ws_message) {
                            let mut tx_lock = tx_conv.lock().await;
                            if tx_lock.send(Message::Text(ws_text.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                }
            }
        });
    }

    Ok(())
}


// Extract JWT token dari query parameter
fn extract_token_from_query(uri: &axum::http::Uri) -> Result<String, AppError> {
    let query = uri.query().ok_or_else(|| AppError::unauthorized("Missing query parameters"))?;

    for (key, value) in form_urlencoded::parse(query.as_bytes()) {
        if key == "token" {
            return Ok(value.into_owned());
        }
    }

    Err(AppError::unauthorized("Missing token parameter"))
}

// Validate JWT token dari query parameter
async fn validate_websocket_token(
    token: &str,
    state: &AppState,
) -> Result<WebSocketParticipant, AppError> {
    let claims = crate::utils::jwt::validate_token(token)
        .map_err(|_| AppError::unauthorized("Token tidak valid atau sudah expired"))?;

    // Validasi role untuk chat service 
    if !claims.is_customer() && !claims.is_seller() {
        return Err(AppError::forbidden("Hanya customer dan seller yang bisa akses chat"));
    }

    // Check user status dari database
    let user_status = sqlx::query_scalar!(
        "SELECT is_active FROM users WHERE id = $1",
        claims.sub
    )
    .fetch_one(&state.db)
    .await
    .map_err(|_| AppError::unauthorized("User tidak ditemukan"))?;

    let is_active = user_status.unwrap_or(false);
    if !is_active {
        return Err(AppError::forbidden("User tidak aktif atau dibanned"));
    }

    // Validate connection limit 
    if !state.ws_limiter.can_add_connection(claims.sub, 3).await {
        return Err(AppError::forbidden("Too many WebSocket connections"));
    }

    tracing::info!("WebSocket connection validated for active user {} ({})", claims.sub, claims.email);

    Ok(WebSocketParticipant {
        user_id: claims.sub,
        email: claims.email,
        role: claims.role,
        is_active,
    })
}

// Handle WebSocket connection
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    Path(conversation_id): Path<i32>,
    State(state): State<AppState>,
    uri: axum::http::Uri,
) -> Result<Response, AppError> {
    // Extract token dari query parameter
    let token = extract_token_from_query(&uri)?;

    // Validate token dan dapatkan participant
    let participant = validate_websocket_token(&token, &state).await?;

    // Validate participant role
    if !participant.role.contains("customer") && !participant.role.contains("seller") {
        return Err(AppError::forbidden("Invalid role for chat access"));
    }

    // Validate participant is active
    if !participant.is_active {
        return Err(AppError::forbidden("Account is not active"));
    }

    // Validate bahwa user adalah participant dalam conversation
    let is_participant = state.conversation_repo
        .is_participant(conversation_id, participant.user_id)
        .await?;

    if !is_participant {
        return Err(AppError::forbidden("Tidak memiliki akses ke conversation ini"));
    }

    // Cek connection limiter
    state.ws_limiter.add_connection(participant.user_id).await;
    tracing::info!("WebSocket connection dimulai untuk user {} ({}) ke conversation {}",
                   participant.user_id, participant.email, conversation_id);

    // Upgrade HTTP ke WebSocket
    Ok(ws.on_upgrade(move |socket| handle_websocket_socket(
        socket,
        participant,
        state,
        conversation_id,
    )))
}

// Handle WebSocket communication
async fn handle_websocket_socket(
    socket: WebSocket,
    participant: WebSocketParticipant,
    state: AppState,
    conversation_id: i32,
) {
    let connection_id = Uuid::new_v4();

    // Buat connection info dengan data lengkap
    let participant_role = participant.role.clone();
    let connection = Arc::new(WsConnection {
        user_id: participant.user_id,
        user_email: participant.email.clone(),
        user_role: participant_role,
        conversation_subscriptions: Arc::new(RwLock::new(HashMap::new())),
        is_alive: Arc::new(RwLock::new(true)),
    });

    // Subscribe ke conversation ini secara otomatis
    {
        let mut subscriptions = connection.conversation_subscriptions.write().await;
        subscriptions.insert(conversation_id, true);
    }

    // Tambahkan koneksi ke ConnectionManager untuk tracking real-time
    ConnectionManager::tambah_koneksi(connection_id, connection.clone()).await;

    tracing::info!("WebSocket koneksi {} dibuat untuk user {} ({}) dengan role {}",
                  connection_id, participant.user_id, participant.email, connection.user_role);

    // Split WebSocket ke sender dan receiver
    let (sender, mut receiver) = socket.split();

    // Clone untuk async tasks
    let tx = Arc::new(Mutex::new(sender));
    let tx_outgoing = tx.clone();
    let tx_incoming = tx.clone();
    let conn_clone = connection.clone();
    let nats_client = state.nats_client.clone();
    let state_clone = state.clone();
    let participant_clone = participant.clone();

    // Handle outgoing messages (server ke client)
    let outgoing_task = {
        let connection_id = connection_id;
        let connection = conn_clone.clone();

        tokio::spawn(async move {
            // Send initial subscription confirmation
            if let Ok(subscribed_msg) = serde_json::to_string(&WsMessage::Subscribed {
                conversation_id
            }) {
                let mut tx_lock = tx_outgoing.lock().await;
                let _ = tx_lock.send(Message::Text(subscribed_msg.into())).await;
            }

            // Setup NATS subscription jika available
            if let Some(nats_client) = &nats_client {
                if let Err(e) = process_nats_messages(
                    nats_client,
                    connection_id,
                    connection.clone(),
                    tx_outgoing.clone(),
                ).await {
                    tracing::error!("NATS subscription setup failed: {}", e);
                }
            } else {
                tracing::warn!("NATS client tidak tersedia, real-time features terbatas");
            }

            // Keep connection alive dengan ping/pong
            let mut ping_interval = tokio::time::interval(std::time::Duration::from_secs(30));

            loop {
                tokio::select! {
                    _ = ping_interval.tick() => {
                        // Check if connection masih alive
                        let is_alive = *connection.is_alive.read().await;
                        if !is_alive {
                            break;
                        }

                        // Send ping
                        let mut tx_lock = tx_outgoing.lock().await;
                        if let Ok(ping_msg) = serde_json::to_string(&WsMessage::Ping) {
                            if tx_lock.send(Message::Text(ping_msg.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                }
            }
        })
    };

    // Handle incoming messages (client ke server)
    let incoming_task = {
        let state = state_clone;
        let connection = conn_clone;
        let participant = participant_clone;

        tokio::spawn(async move {
            while let Some(msg_result) = receiver.next().await {
                match msg_result {
                    Ok(Message::Text(text)) => {
                        if let Err(e) = handle_text_message(
                            &text,
                            &connection,
                            &participant,
                            &state,
                            connection_id,
                        ).await {
                            tracing::error!("Error handling message from connection {}: {}", connection_id, e);

                            // Send error response
                            if let Ok(error_msg) = serde_json::to_string(&WsMessage::Error {
                                code: "MESSAGE_ERROR".to_string(),
                                message: e.to_string(),
                            }) {
                                let mut tx_lock = tx_incoming.lock().await;
                                let _ = tx_lock.send(Message::Text(error_msg.into())).await;
                            }
                        }
                    }
                    Ok(Message::Close(close_frame)) => {
                        tracing::info!("WebSocket connection {} closed by client: {:?}",
                                     connection_id, close_frame);
                        break;
                    }
                    Ok(Message::Ping(data)) => {
                        let mut tx_lock = tx_incoming.lock().await;
                        let _ = tx_lock.send(Message::Pong(data)).await;
                    }
                    Ok(Message::Pong(_)) => {
                        // Update alive status
                        *connection.is_alive.write().await = true;
                    }
                    Err(e) => {
                        tracing::error!("WebSocket error untuk connection {}: {}", connection_id, e);
                        break;
                    }
                    _ => {} // Ignore other message types
                }
            }
        })
    };

    // Tunggu salah satu task selesai
    tokio::select! {
        _ = outgoing_task => {
            tracing::info!("Outgoing task selesai untuk connection {}", connection_id);
        }
        _ = incoming_task => {
            tracing::info!("Incoming task selesai untuk connection {}", connection_id);
        }
    }

    // Hapus dari connection limiter
    state.ws_limiter.remove_connection(participant.user_id).await;

    // Hapus koneksi dari ConnectionManager untuk tracking real-time
    ConnectionManager::hapus_koneksi(&connection_id).await;

    tracing::info!("WebSocket koneksi {} ditutup untuk user {} ({}), total koneksi aktif: {}",
                  connection_id, participant.user_id, participant.email,
                  ConnectionManager::total_koneksi().await);
}

// Handle incoming text messages dari client
async fn handle_text_message(
    text: &str,
    connection: &WsConnection,
    participant: &WebSocketParticipant,
    state: &AppState,
    connection_id: Uuid,
) -> Result<(), AppError> {
    // Log pesan masuk dengan connection ID untuk debugging
    tracing::debug!("Received WebSocket message from connection {}: {}", connection_id, text);

    // validate message
    let ws_message: WsMessage = serde_json::from_str(text)
        .map_err(|e| {
            tracing::warn!("WebSocket protocol error from connection {}: {} - {}", connection_id, text, e);
            AppError::websocket(format!("Invalid message format: {}", e))
        })?;

    match ws_message {
        WsMessage::Subscribe { conversation_id } => {
            // Validate participant access
            let is_participant = state.conversation_repo
                .is_participant(conversation_id, participant.user_id)
                .await?;

            if !is_participant {
                return Err(AppError::forbidden("Tidak memiliki akses ke conversation ini"));
            }

            // Add subscription
            {
                let mut subscriptions = connection.conversation_subscriptions.write().await;
                subscriptions.insert(conversation_id, true);
            }

            // Subscribe ke NATS untuk conversation baru
            if let Some(nats_client) = &state.nats_client {
                let subject = format!("chat.{}", conversation_id);
                if let Err(e) = nats_client.subscribe(subject).await {
                    tracing::warn!("Gagal subscribe ke conversation {}: {}", conversation_id, e);
                }
            }

            tracing::info!("Connection {} - User {} ({}) subscribe ke conversation {}",
                           connection_id, participant.user_id, participant.email, conversation_id);
        }

        WsMessage::Unsubscribe { conversation_id } => {
            // Remove subscription
            {
                let mut subscriptions = connection.conversation_subscriptions.write().await;
                subscriptions.remove(&conversation_id);
            }

            tracing::info!("Connection {} - User {} ({}) unsubscribe dari conversation {}",
                           connection_id, participant.user_id, participant.email, conversation_id);
        }

        WsMessage::TypingStart { conversation_id } => {
            // Validate participant access
            let is_participant = state.conversation_repo
                .is_participant(conversation_id, participant.user_id)
                .await?;

            if !is_participant {
                return Err(AppError::forbidden("Tidak memiliki akses ke conversation ini"));
            }

            // Create typing indicator using WsConnection fields
            let typing_indicator = TypingIndicator::new(
                conversation_id,
                participant.user_id,
                connection.user_email.clone(),
                connection.user_role.clone(),
                true,
            );

            // Broadcast typing notification
            if let Some(nats_client) = &state.nats_client {
                let subject = format!("chat.{}", conversation_id);
                if let Err(e) = nats_client
                    .publish(subject, typing_indicator.to_nats_payload().into())
                    .await
                {
                    tracing::warn!("Gagal broadcast typing start: {}", e);
                }

                tracing::info!("User {} ({}) started typing in conversation {}",
                    participant.user_id, connection.user_email, conversation_id);
            }
        }

        WsMessage::TypingStop { conversation_id } => {
            // Validate participant access
            let is_participant = state.conversation_repo
                .is_participant(conversation_id, participant.user_id)
                .await?;

            if !is_participant {
                return Err(AppError::forbidden("Tidak memiliki akses ke conversation ini"));
            }

            // menampilkan typing indicator menggunakan field dari WsConnection
            let typing_indicator = TypingIndicator::new(
                conversation_id,
                participant.user_id,
                connection.user_email.clone(),
                connection.user_role.clone(),
                false,
            );

            // Broadcast typing stop notification
            if let Some(nats_client) = &state.nats_client {
                let subject = format!("chat.{}", conversation_id);
                if let Err(e) = nats_client
                    .publish(subject, typing_indicator.to_nats_payload().into())
                    .await
                {
                    tracing::warn!("Gagal broadcast typing stop: {}", e);
                }

                tracing::info!("User {} ({}) stopped typing in conversation {}",
                    participant.user_id, connection.user_email, conversation_id);
            }
        }

        WsMessage::Ping => {
            // Handle ping dengan pong
            // Pong handling sudah ada di main loop
        }

        _ => {
            return Err(AppError::bad_request("Unsupported WebSocket message type"));
        }
    }

    Ok(())
}