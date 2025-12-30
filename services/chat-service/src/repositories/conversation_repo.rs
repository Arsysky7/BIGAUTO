// Repository untuk Conversation operations
use crate::domain::{Conversation, CreateConversationRequest};
use anyhow::Result;
use sqlx::PgPool;

// Repository untuk conversation database operations
#[derive(Clone)]
pub struct ConversationRepository {
    pool: PgPool,
}

impl ConversationRepository {
    // membuat new conversation repository
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // membuat conversation baru
    pub async fn create_conversation(
        &self,
        customer_id: i32,
        request: CreateConversationRequest,
    ) -> Result<i32, sqlx::Error> {
        let result = sqlx::query!(
            r#"
            INSERT INTO conversations (customer_id, seller_id, vehicle_id)
            VALUES ($1, $2, $3)
            ON CONFLICT (customer_id, seller_id, vehicle_id)
            DO NOTHING
            RETURNING id
            "#,
            customer_id,
            request.seller_id,
            request.vehicle_id
        )
        .fetch_one(&self.pool)
        .await;

        match result {
            Ok(record) => Ok(record.id),
            Err(sqlx::Error::RowNotFound) => {
                let existing = sqlx::query_scalar!(
                    "SELECT id FROM conversations WHERE customer_id = $1 AND seller_id = $2 AND vehicle_id = $3",
                    customer_id,
                    request.seller_id,
                    request.vehicle_id
                )
                .fetch_one(&self.pool)
                .await?;
                Ok(existing)
            }
            Err(e) => Err(e),
        }
    }

    // Get conversation by ID
    pub async fn get_conversation_by_id(
        &self,
        conversation_id: i32,
        user_id: i32,
    ) -> Result<Option<Conversation>, sqlx::Error> {
        let row = sqlx::query!(
            "SELECT id, customer_id, seller_id, vehicle_id, last_message, last_message_at, created_at, updated_at
             FROM conversations WHERE id = $1 AND (customer_id = $2 OR seller_id = $2)",
            conversation_id,
            user_id
        )
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(record) => Ok(Some(Conversation {
                id: record.id,
                customer_id: record.customer_id,
                seller_id: record.seller_id,
                vehicle_id: record.vehicle_id,
                last_message: record.last_message,
                last_message_at: record.last_message_at,
                created_at: record.created_at.unwrap_or_else(|| chrono::Utc::now()),
                updated_at: record.updated_at.unwrap_or_else(|| chrono::Utc::now()),
            })),
            None => Ok(None),
        }
    }

    // Get conversations for user
    pub async fn get_user_conversations(
        &self,
        user_id: i32,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Conversation>, sqlx::Error> {
        let rows = sqlx::query!(
            "SELECT id, customer_id, seller_id, vehicle_id, last_message, last_message_at, created_at, updated_at
             FROM conversations WHERE customer_id = $1 OR seller_id = $1
             ORDER BY updated_at DESC LIMIT $2 OFFSET $3",
            user_id,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;

        let conversations = rows.into_iter().map(|record| Conversation {
            id: record.id,
            customer_id: record.customer_id,
            seller_id: record.seller_id,
            vehicle_id: record.vehicle_id,
            last_message: record.last_message,
            last_message_at: record.last_message_at,
            created_at: record.created_at.unwrap_or_else(|| chrono::Utc::now()),
            updated_at: record.updated_at.unwrap_or_else(|| chrono::Utc::now()),
        }).collect();

        Ok(conversations)
    }

    // Update last message info
    pub async fn update_last_message(
        &self,
        conversation_id: i32,
        last_message: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE conversations SET last_message = $1, last_message_at = NOW(), updated_at = NOW() WHERE id = $2",
            last_message,
            conversation_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // Mengambil percakapan beserta detail (informasi) para pesertanya
    pub async fn get_conversation_with_details(
        &self,
        conversation_id: i32,
        current_user_id: i32,
    ) -> Result<Option<ConversationWithDetails>, sqlx::Error> {
        let result = sqlx::query!(
            r#"
            SELECT
                c.id, c.customer_id, c.seller_id, c.vehicle_id,
                c.last_message, c.last_message_at, c.created_at, c.updated_at,
                cu.name as customer_name,
                su.name as seller_name,
                v.title as vehicle_title,
                (SELECT COUNT(*) FROM messages m
                 WHERE m.conversation_id = c.id AND m.sender_id != $2 AND m.is_read = false) as unread_messages
            FROM conversations c
            JOIN users cu ON c.customer_id = cu.id
            JOIN users su ON c.seller_id = su.id
            LEFT JOIN vehicles v ON c.vehicle_id = v.id
            WHERE c.id = $1 AND (c.customer_id = $2 OR c.seller_id = $2)
            "#,
            conversation_id,
            current_user_id
        )
        .fetch_optional(&self.pool)
        .await?;

        match result {
            Some(record) => {
                let conversation = Conversation {
                    id: record.id,
                    customer_id: record.customer_id,
                    seller_id: record.seller_id,
                    vehicle_id: record.vehicle_id,
                    last_message: record.last_message,
                    last_message_at: record.last_message_at,
                    created_at: record.created_at.unwrap_or_else(|| chrono::Utc::now()),
                    updated_at: record.updated_at.unwrap_or_else(|| chrono::Utc::now()),
                };

                let details = ConversationWithDetails {
                    conversation,
                    customer_name: record.customer_name,
                    seller_name: record.seller_name,
                    vehicle_title: Some(record.vehicle_title),
                    unread_messages: record.unread_messages.unwrap_or(0),
                };

                Ok(Some(details))
            }
            None => Ok(None),
        }
    }

    // Check if user is participant in conversation
    pub async fn is_participant(
        &self,
        conversation_id: i32,
        user_id: i32,
    ) -> Result<bool, sqlx::Error> {
        let exists = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM conversations WHERE id = $1 AND (customer_id = $2 OR seller_id = $2))",
            conversation_id,
            user_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(exists.unwrap_or(false))
    }

    // Get conversation between specific users and vehicle
    pub async fn find_conversation(
        &self,
        customer_id: i32,
        seller_id: i32,
        vehicle_id: Option<i32>,
    ) -> Result<Option<Conversation>, sqlx::Error> {
        // Query berdasarkan vehicle_id yang diberikan
        if let Some(vid) = vehicle_id {
            let row = sqlx::query!(
                "SELECT id, customer_id, seller_id, vehicle_id, last_message, last_message_at, created_at, updated_at
                 FROM conversations WHERE customer_id = $1 AND seller_id = $2 AND vehicle_id = $3",
                customer_id,
                seller_id,
                vid
            )
            .fetch_optional(&self.pool)
            .await?;

            match row {
                Some(record) => Ok(Some(Conversation {
                    id: record.id,
                    customer_id: record.customer_id,
                    seller_id: record.seller_id,
                    vehicle_id: record.vehicle_id,
                    last_message: record.last_message,
                    last_message_at: record.last_message_at,
                    created_at: record.created_at.unwrap_or_else(|| chrono::Utc::now()),
                    updated_at: record.updated_at.unwrap_or_else(|| chrono::Utc::now()),
                })),
                None => Ok(None),
            }
        } else {
            let row = sqlx::query!(
                "SELECT id, customer_id, seller_id, vehicle_id, last_message, last_message_at, created_at, updated_at
                 FROM conversations WHERE customer_id = $1 AND seller_id = $2 AND vehicle_id IS NULL",
                customer_id,
                seller_id
            )
            .fetch_optional(&self.pool)
            .await?;

            match row {
                Some(record) => Ok(Some(Conversation {
                    id: record.id,
                    customer_id: record.customer_id,
                    seller_id: record.seller_id,
                    vehicle_id: record.vehicle_id,
                    last_message: record.last_message,
                    last_message_at: record.last_message_at,
                    created_at: record.created_at.unwrap_or_else(|| chrono::Utc::now()),
                    updated_at: record.updated_at.unwrap_or_else(|| chrono::Utc::now()),
                })),
                None => Ok(None),
            }
        }
    }

    // Mark messages as read for user
    pub async fn mark_messages_as_read(
        &self,
        conversation_id: i32,
        user_id: i32,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!(
            "UPDATE messages SET is_read = true, read_at = NOW()
             WHERE conversation_id = $1 AND sender_id != $2 AND is_read = false",
            conversation_id,
            user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    // Get unread message count for user
    pub async fn get_unread_count(
        &self,
        user_id: i32,
    ) -> Result<i64, sqlx::Error> {
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM messages m
             JOIN conversations c ON m.conversation_id = c.id
             WHERE (c.customer_id = $1 OR c.seller_id = $1) AND m.sender_id != $1 AND m.is_read = false",
            user_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(count.unwrap_or(0))
    }
}

// Additional struct for conversation with details
#[derive(Debug, Clone)]
pub struct ConversationWithDetails {
    pub conversation: Conversation,
    pub customer_name: String,
    pub seller_name: String,
    pub vehicle_title: Option<String>,
    pub unread_messages: i64,
}