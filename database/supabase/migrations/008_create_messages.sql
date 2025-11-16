-- PPL/database/supabase/migrations/008_create_messages.sql

-- Table: messages
-- Deskripsi: Chat messages

CREATE TABLE messages (
    id SERIAL PRIMARY KEY,
    conversation_id INT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    sender_id INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    
    -- Message content
    content TEXT NOT NULL,
    message_type VARCHAR(20) DEFAULT 'text' CHECK (message_type IN ('text', 'image')),
    
    -- Media (jika type = image)
    media_url VARCHAR(500),
    thumbnail_url VARCHAR(500),
    
    -- Read status
    is_read BOOLEAN DEFAULT false,
    read_at TIMESTAMPTZ,
    
    created_at TIMESTAMPTZ DEFAULT NOW()
);