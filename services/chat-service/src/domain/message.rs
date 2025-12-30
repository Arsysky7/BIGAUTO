// Domain model untuk Message
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Message {
    pub id: i32,
    pub conversation_id: i32,
    pub sender_id: i32,
    pub content: String,
    pub message_type: MessageType,
    pub media_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub is_read: bool,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
pub enum MessageType {
    Text,
    Image,
}

impl MessageType {
    // Konversi dari string untuk database
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "image" => MessageType::Image,
            _ => MessageType::Text,
        }
    }

    // Konversi dari Option<String> untuk database
    pub fn from_str_option(s: &Option<String>) -> Self {
        match s.as_ref().map(|s| s.to_lowercase()).unwrap_or_else(|| "text".to_string()).as_str() {
            "image" => MessageType::Image,
            _ => MessageType::Text,
        }
    }

    // Konversi ke string untuk database
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageType::Text => "text",
            MessageType::Image => "image",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateMessageRequest {
    pub conversation_id: i32,
    pub content: String,
    pub message_type: Option<String>,
    pub media_url: Option<String>,
    pub thumbnail_url: Option<String>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageResponse {
    pub id: i32,
    pub conversation_id: i32,
    pub sender_id: i32,
    pub sender_name: String,
    pub content: String,
    pub message_type: String,
    pub media_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub is_read: bool,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketMessage {
    pub r#type: String, 
    pub conversation_id: i32,
    pub sender_id: i32,
    pub data: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypingIndicator {
    pub conversation_id: i32,
    pub user_id: i32,
    pub user_email: String,
    pub user_role: String,
    pub is_typing: bool,
}

impl TypingIndicator {
    // Create new typing indicator
    pub fn new(
        conversation_id: i32,
        user_id: i32,
        user_email: String,
        user_role: String,
        is_typing: bool,
    ) -> Self {
        Self {
            conversation_id,
            user_id,
            user_email,
            user_role,
            is_typing,
        }
    }

    // Convert to WebSocket message
    pub fn to_websocket_message(&self) -> WebSocketMessage {
        WebSocketMessage {
            r#type: "typing_indicator".to_string(),
            conversation_id: self.conversation_id,
            sender_id: self.user_id,
            data: serde_json::json!({
                "user_id": self.user_id,
                "user_email": self.user_email,
                "user_role": self.user_role,
                "is_typing": self.is_typing,
                "timestamp": chrono::Utc::now()
            }),
            timestamp: chrono::Utc::now(),
        }
    }

    // Convert to NATS payload
    pub fn to_nats_payload(&self) -> String {
        serde_json::json!({
            "type": "user_typing",
            "conversation_id": self.conversation_id,
            "user_id": self.user_id,
            "user_email": self.user_email,
            "user_role": self.user_role,
            "is_typing": self.is_typing
        }).to_string()
    }
}

impl Message {
    // membuat message baru 
    pub fn new(
        conversation_id: i32,
        sender_id: i32,
        content: String,
        message_type: MessageType,
    ) -> Self {
        Self {
            id: 0,
            conversation_id,
            sender_id,
            content,
            message_type,
            media_url: None,
            thumbnail_url: None,
            is_read: false,
            read_at: None,
            created_at: Utc::now(),
        }
    }

    // Membuat message dengan media attachment
    pub fn new_with_media(
        conversation_id: i32,
        sender_id: i32,
        content: String,
        message_type: MessageType,
        media_url: Option<String>,
        thumbnail_url: Option<String>,
    ) -> Self {
        let mut message = Self::new(
            conversation_id,
            sender_id,
            content,
            message_type,
        );
        message.media_url = media_url;
        message.thumbnail_url = thumbnail_url;
        message
    }

    // mark as read
    pub fn mark_as_read(&mut self) {
        self.is_read = true;
        self.read_at = Some(Utc::now());
    }

    // Convert ke WebSocket message
    pub fn to_websocket_message(&self, sender_name: String) -> WebSocketMessage {
        WebSocketMessage {
            r#type: "message".to_string(),
            conversation_id: self.conversation_id,
            sender_id: self.sender_id,
            data: serde_json::json!({
                "id": self.id,
                "sender_name": sender_name,
                "content": self.content,
                "message_type": self.message_type.as_str(),
                "media_url": self.media_url,
                "thumbnail_url": self.thumbnail_url,
                "is_read": self.is_read,
                "created_at": self.created_at
            }),
            timestamp: self.created_at,
        }
    }

    // Convert ke MessageResponse untuk API consistency
    pub fn to_response(&self, sender_name: String) -> MessageResponse {
        MessageResponse {
            id: self.id,
            conversation_id: self.conversation_id,
            sender_id: self.sender_id,
            sender_name,
            content: self.content.clone(),
            message_type: self.message_type.as_str().to_string(),
            media_url: self.media_url.clone(),
            thumbnail_url: self.thumbnail_url.clone(),
            is_read: self.is_read,
            read_at: self.read_at,
            created_at: self.created_at,
        }
    }

    // Validasi message content
    pub fn is_valid(&self) -> bool {
        !self.content.trim().is_empty() && self.content.len() <= 2000
    }
}