// Domain model untuk Conversation
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Conversation {
    pub id: i32,
    pub customer_id: i32,
    pub seller_id: i32,
    pub vehicle_id: Option<i32>,
    pub last_message: Option<String>,
    pub last_message_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateConversationRequest {
    pub seller_id: i32,
    pub vehicle_id: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConversationResponse {
    pub id: i32,
    pub customer_id: i32,
    pub seller_id: i32,
    pub seller_name: String,
    pub vehicle_id: Option<i32>,
    pub vehicle_title: Option<String>,
    pub last_message: Option<String>,
    pub last_message_at: Option<DateTime<Utc>>,
    pub unread_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationWithDetails {
    pub conversation: Conversation,
    pub customer_name: String,
    pub seller_name: String,
    pub vehicle_title: Option<String>,
    pub unread_messages: i64,
}

impl ConversationWithDetails {
    // Buat ConversationWithDetails dari data query
    pub fn from_query_data(
        conversation: Conversation,
        customer_name: String,
        seller_name: String,
        vehicle_title: Option<String>,
        unread_messages: i64,
    ) -> Self {
        Self {
            conversation,
            customer_name,
            seller_name,
            vehicle_title,
            unread_messages,
        }
    }

    // Getter methods untuk memudahkan akses
    pub fn id(&self) -> i32 {
        self.conversation.id
    }

    pub fn customer_id(&self) -> i32 {
        self.conversation.customer_id
    }

    pub fn seller_id(&self) -> i32 {
        self.conversation.seller_id
    }

    pub fn vehicle_id(&self) -> Option<i32> {
        self.conversation.vehicle_id
    }

    pub fn last_message(&self) -> Option<String> {
        self.conversation.last_message.clone()
    }

    pub fn last_message_at(&self) -> Option<DateTime<Utc>> {
        self.conversation.last_message_at
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.conversation.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.conversation.updated_at
    }

    // Convert ke HashMap untuk response yang flexible
    pub fn to_response_map(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id(),
            "customer_id": self.customer_id(),
            "seller_id": self.seller_id(),
            "customer_name": self.customer_name,
            "seller_name": self.seller_name,
            "vehicle_id": self.vehicle_id(),
            "vehicle_title": self.vehicle_title,
            "last_message": self.last_message(),
            "last_message_at": self.last_message_at(),
            "unread_count": self.unread_messages,
            "created_at": self.created_at(),
            "updated_at": self.updated_at(),
        })
    }
}

impl Conversation {
    // Membuat conversation baru
    pub fn new(customer_id: i32, seller_id: i32, vehicle_id: Option<i32>) -> Self {
        let now = Utc::now();
        Self {
            id: 0, 
            customer_id,
            seller_id,
            vehicle_id,
            last_message: None,
            last_message_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    // Update last message info
    pub fn update_last_message(&mut self, message: &str) {
        self.last_message = Some(message.to_string());
        self.last_message_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    // Check apakah user adalah participant
    pub fn is_participant(&self, user_id: i32) -> bool {
        self.customer_id == user_id || self.seller_id == user_id
    }

    // Get participant lainnya
    pub fn get_other_participant_id(&self, current_user_id: i32) -> Option<i32> {
        if self.customer_id == current_user_id {
            Some(self.seller_id)
        } else if self.seller_id == current_user_id {
            Some(self.customer_id)
        } else {
            None
        }
    }
}