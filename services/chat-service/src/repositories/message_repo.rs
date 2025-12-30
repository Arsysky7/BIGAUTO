// Repository untuk Message operations
use crate::domain::{Message, MessageType, CreateMessageRequest};
use anyhow::Result;
use sqlx::PgPool;

// Repository untuk message database operations
#[derive(Clone)]
pub struct MessageRepository {
    pool: PgPool,
}

impl MessageRepository {
    // Create new message repository
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // Create new message
    pub async fn create_message(
        &self,
        conversation_id: i32,
        sender_id: i32,
        request: CreateMessageRequest,
    ) -> Result<Message, sqlx::Error> {
        // Convert message type dari string ke enum
        let message_type = request.message_type
            .map(|t| MessageType::from_str_option(&Some(t)))
            .unwrap_or(MessageType::Text);

        // Validate message content
        if request.content.trim().is_empty() {
            return Err(sqlx::Error::Protocol("Message content cannot be empty".to_string()));
        }

        if request.content.len() > 2000 {
            return Err(sqlx::Error::Protocol("Message content too long".to_string()));
        }

        let row = sqlx::query!(
            r#"
            INSERT INTO messages (conversation_id, sender_id, content, message_type, media_url, thumbnail_url)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, conversation_id, sender_id, content, message_type, media_url, thumbnail_url, is_read, read_at, created_at
            "#,
            conversation_id,
            sender_id,
            request.content,
            message_type.as_str() as &str,
            request.media_url,
            request.thumbnail_url
        )
        .fetch_one(&self.pool)
        .await?;

        let message = Message {
            id: row.id,
            conversation_id: row.conversation_id,
            sender_id: row.sender_id,
            content: row.content,
            message_type: MessageType::from_str_option(&row.message_type),
            media_url: row.media_url,
            thumbnail_url: row.thumbnail_url,
            is_read: row.is_read.unwrap_or(false),
            read_at: row.read_at,
            created_at: row.created_at.unwrap_or_else(|| chrono::Utc::now()),
        };

        Ok(message)
    }

    // Get messages untuk conversation dengan pagination dan validasi akses user
    pub async fn get_conversation_messages(
        &self,
        conversation_id: i32,
        user_id: i32,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Message>, sqlx::Error> {
        // Validasi bahwa user adalah participant dalam conversation
        let is_participant = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM conversations
             WHERE id = $1 AND (customer_id = $2 OR seller_id = $2)",
            conversation_id,
            user_id
        )
        .fetch_one(&self.pool)
        .await?;

        if is_participant.unwrap_or(0) == 0 {
            return Err(sqlx::Error::Protocol("Access denied: not a conversation participant".to_string()));
        }

        let rows = sqlx::query!(
            "SELECT id, conversation_id, sender_id, content, message_type, media_url, thumbnail_url, is_read, read_at, created_at
             FROM messages WHERE conversation_id = $1 ORDER BY created_at ASC LIMIT $2 OFFSET $3",
            conversation_id,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;

        let messages = rows.into_iter().map(|record| Message {
            id: record.id,
            conversation_id: record.conversation_id,
            sender_id: record.sender_id,
            content: record.content,
            message_type: MessageType::from_str_option(&record.message_type),
            media_url: record.media_url,
            thumbnail_url: record.thumbnail_url,
            is_read: record.is_read.unwrap_or(false),
            read_at: record.read_at,
            created_at: record.created_at.unwrap_or_else(|| chrono::Utc::now()),
        }).collect();

        Ok(messages)
    }

    // Get message by ID
    pub async fn get_message_by_id(
        &self,
        message_id: i32,
        user_id: i32,
    ) -> Result<Option<Message>, sqlx::Error> {
        let row = sqlx::query!(
            r#"
            SELECT m.id, m.conversation_id, m.sender_id, m.content, m.message_type,
                   m.media_url, m.thumbnail_url, m.is_read, m.read_at, m.created_at
            FROM messages m
            JOIN conversations c ON m.conversation_id = c.id
            WHERE m.id = $1 AND (c.customer_id = $2 OR c.seller_id = $2)
            "#,
            message_id,
            user_id
        )
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(record) => Ok(Some(Message {
                id: record.id,
                conversation_id: record.conversation_id,
                sender_id: record.sender_id,
                content: record.content,
                message_type: MessageType::from_str_option(&record.message_type),
                media_url: record.media_url,
                thumbnail_url: record.thumbnail_url,
                is_read: record.is_read.unwrap_or(false),
                read_at: record.read_at,
                created_at: record.created_at.unwrap_or_else(|| chrono::Utc::now()),
            })),
            None => Ok(None),
        }
    }

    // Mark message as read
    pub async fn mark_message_as_read(
        &self,
        message_id: i32,
        user_id: i32,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE messages SET is_read = true, read_at = NOW()
             WHERE id = $1 AND sender_id != $2 AND is_read = false",
            message_id,
            user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // Mark all messages as read in conversation
    pub async fn mark_conversation_read(
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

    // Get unread message count untuk conversation
    pub async fn get_conversation_unread_count(
        &self,
        conversation_id: i32,
        user_id: i32,
    ) -> Result<i64, sqlx::Error> {
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM messages
             WHERE conversation_id = $1 AND sender_id != $2 AND is_read = false",
            conversation_id,
            user_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(count.unwrap_or(0))
    }

    // Delete message (hanya oleh sender)
    pub async fn delete_message(
        &self,
        message_id: i32,
        user_id: i32,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!(
            "DELETE FROM messages WHERE id = $1 AND sender_id = $2 RETURNING id",
            message_id,
            user_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.is_some())
    }

    // Get latest message untuk conversation
    pub async fn get_latest_message(
        &self,
        conversation_id: i32,
    ) -> Result<Option<Message>, sqlx::Error> {
        let row = sqlx::query!(
            "SELECT id, conversation_id, sender_id, content, message_type, media_url, thumbnail_url, is_read, read_at, created_at
             FROM messages WHERE conversation_id = $1 ORDER BY created_at DESC LIMIT 1",
            conversation_id
        )
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(record) => Ok(Some(Message {
                id: record.id,
                conversation_id: record.conversation_id,
                sender_id: record.sender_id,
                content: record.content,
                message_type: MessageType::from_str_option(&record.message_type),
                media_url: record.media_url,
                thumbnail_url: record.thumbnail_url,
                is_read: record.is_read.unwrap_or(false),
                read_at: record.read_at,
                created_at: record.created_at.unwrap_or_else(|| chrono::Utc::now()),
            })),
            None => Ok(None),
        }
    }

    // Get messages by sender
    pub async fn get_messages_by_sender(
        &self,
        conversation_id: i32,
        sender_id: i32,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Message>, sqlx::Error> {
        let rows = sqlx::query!(
            "SELECT id, conversation_id, sender_id, content, message_type, media_url, thumbnail_url, is_read, read_at, created_at
             FROM messages WHERE conversation_id = $1 AND sender_id = $2 ORDER BY created_at DESC LIMIT $3 OFFSET $4",
            conversation_id,
            sender_id,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;

        let messages = rows.into_iter().map(|record| Message {
            id: record.id,
            conversation_id: record.conversation_id,
            sender_id: record.sender_id,
            content: record.content,
            message_type: MessageType::from_str_option(&record.message_type),
            media_url: record.media_url,
            thumbnail_url: record.thumbnail_url,
            is_read: record.is_read.unwrap_or(false),
            read_at: record.read_at,
            created_at: record.created_at.unwrap_or_else(|| chrono::Utc::now()),
        }).collect();

        Ok(messages)
    }

    // Count total messages dalam conversation
    pub async fn get_conversation_message_count(
        &self,
        conversation_id: i32,
    ) -> Result<i64, sqlx::Error> {
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM messages WHERE conversation_id = $1",
            conversation_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(count.unwrap_or(0))
    }

    // Get media messages (images, files) dalam conversation
    pub async fn get_media_messages(
        &self,
        conversation_id: i32,
        user_id: i32,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Message>, sqlx::Error> {
        let rows = sqlx::query!(
            r#"
            SELECT m.id, m.conversation_id, m.sender_id, m.content, m.message_type,
                   m.media_url, m.thumbnail_url, m.is_read, m.read_at, m.created_at
            FROM messages m
            JOIN conversations c ON m.conversation_id = c.id
            WHERE m.conversation_id = $1
            AND (c.customer_id = $2 OR c.seller_id = $2)
            AND m.message_type != 'text'
            ORDER BY m.created_at DESC LIMIT $3 OFFSET $4
            "#,
            conversation_id,
            user_id,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;

        let messages = rows.into_iter().map(|record| Message {
            id: record.id,
            conversation_id: record.conversation_id,
            sender_id: record.sender_id,
            content: record.content,
            message_type: MessageType::from_str_option(&record.message_type),
            media_url: record.media_url,
            thumbnail_url: record.thumbnail_url,
            is_read: record.is_read.unwrap_or(false),
            read_at: record.read_at,
            created_at: record.created_at.unwrap_or_else(|| chrono::Utc::now()),
        }).collect();

        Ok(messages)
    }

    // Search messages dalam conversation
    pub async fn search_conversation_messages(
        &self,
        conversation_id: i32,
        user_id: i32,
        search_query: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Message>, sqlx::Error> {
        let search_pattern = format!("%{}%", search_query);

        let rows = sqlx::query!(
            r#"
            SELECT m.id, m.conversation_id, m.sender_id, m.content, m.message_type,
                   m.media_url, m.thumbnail_url, m.is_read, m.read_at, m.created_at
            FROM messages m
            JOIN conversations c ON m.conversation_id = c.id
            WHERE m.conversation_id = $1
            AND (c.customer_id = $2 OR c.seller_id = $2)
            AND m.content ILIKE $3
            ORDER BY m.created_at DESC LIMIT $4 OFFSET $5
            "#,
            conversation_id,
            user_id,
            search_pattern,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;

        let messages = rows.into_iter().map(|record| Message {
            id: record.id,
            conversation_id: record.conversation_id,
            sender_id: record.sender_id,
            content: record.content,
            message_type: MessageType::from_str_option(&record.message_type),
            media_url: record.media_url,
            thumbnail_url: record.thumbnail_url,
            is_read: record.is_read.unwrap_or(false),
            read_at: record.read_at,
            created_at: record.created_at.unwrap_or_else(|| chrono::Utc::now()),
        }).collect();

        Ok(messages)
    }
}