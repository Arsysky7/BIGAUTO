-- PPL/database/supabase/migrations/010_create_seller_balance.sql

-- Table: seller_balance
-- Deskripsi: Saldo seller

CREATE TABLE seller_balance (
    id SERIAL PRIMARY KEY,
    seller_id INT UNIQUE NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    
    available_balance DECIMAL(12, 2) DEFAULT 0.0,
    pending_balance DECIMAL(12, 2) DEFAULT 0.0,
    total_earned DECIMAL(12, 2) DEFAULT 0.0,
    
    updated_at TIMESTAMPTZ DEFAULT NOW()
);