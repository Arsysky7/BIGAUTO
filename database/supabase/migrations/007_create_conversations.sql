-- PPL/database/supabase/migrations/007_create_conversations.sql
-- Table: conversations
-- Deskripsi: Chat conversations (NATS)

CREATE TABLE conversations (
    id SERIAL PRIMARY KEY,
    customer_id INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    seller_id INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    vehicle_id INT REFERENCES vehicles(id) ON DELETE SET NULL,
    
    -- Last message preview
    last_message TEXT,
    last_message_at TIMESTAMPTZ,
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    
    -- 1 customer + 1 seller + 1 vehicle = 1 conversation
    UNIQUE(customer_id, seller_id, vehicle_id)
);