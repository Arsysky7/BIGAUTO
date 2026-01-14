use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use utoipa::ToSchema;

// Model notification dari database
#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct Notification {
    pub id: i32,
    pub user_id: i32,
    pub notification_type: String,
    pub title: String,
    pub message: String,
    pub related_id: Option<i32>,
    pub related_type: Option<String>,
    pub is_read: bool,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

// Response notification list untuk user
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct NotificationResponse {
    pub id: i32,
    pub notification_type: String,
    pub title: String,
    pub message: String,
    pub related_id: Option<i32>,
    pub related_type: Option<String>,
    pub is_read: bool,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<Notification> for NotificationResponse {
    fn from(notif: Notification) -> Self {
        Self {
            id: notif.id,
            notification_type: notif.notification_type,
            title: notif.title,
            message: notif.message,
            related_id: notif.related_id,
            related_type: notif.related_type,
            is_read: notif.is_read,
            read_at: notif.read_at,
            created_at: notif.created_at,
        }
    }
}

// Response untuk mark as read
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MarkReadResponse {
    pub message: String,
}

// Response untuk read all
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ReadAllResponse {
    pub message: String,
    pub affected_count: i64,
}

// Unread count response
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UnreadCountResponse {
    pub unread_count: i64,
}
