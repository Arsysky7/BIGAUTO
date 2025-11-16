-- PPL/database/supabase/migrations/014_create_user_sessions.sql
-- Table: user_sessions
-- Deskripsi: Refresh token & session management

CREATE TABLE user_sessions (
    id SERIAL PRIMARY KEY,
    user_id INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    
    -- Token info
    refresh_token VARCHAR(500) UNIQUE NOT NULL,
    access_token_jti VARCHAR(255),
    
    -- Device info
    user_agent TEXT,
    ip_address INET,
    device_name VARCHAR(255),
    
    -- Validity
    expires_at TIMESTAMPTZ NOT NULL,
    last_activity TIMESTAMPTZ DEFAULT NOW(),
    is_active BOOLEAN DEFAULT true,
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Index untuk performa
CREATE INDEX idx_user_sessions_user_id ON user_sessions(user_id);
CREATE INDEX idx_user_sessions_refresh_token ON user_sessions(refresh_token);
CREATE INDEX idx_user_sessions_expires_at ON user_sessions(expires_at);