-- / PPL/database/supabase/migrations/022_create_rate_limits.sql
-- Table: rate_limits
-- Deskripsi: Track rate limiting per user/IP

CREATE TABLE rate_limits (
    id SERIAL PRIMARY KEY,
    
    -- Identifier
    identifier VARCHAR(255) NOT NULL, -- bisa user_id atau IP
    identifier_type VARCHAR(20) NOT NULL CHECK (
        identifier_type IN ('user', 'ip', 'email')
    ),
    
    -- Action being limited
    action VARCHAR(100) NOT NULL,
    
    -- Counters
    request_count INT DEFAULT 1,
    window_start TIMESTAMPTZ DEFAULT NOW(),
    window_end TIMESTAMPTZ NOT NULL,
    
    -- Blocking
    is_blocked BOOLEAN DEFAULT false,
    blocked_until TIMESTAMPTZ,
    
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    
    UNIQUE(identifier, identifier_type, action, window_start)
);

-- Index untuk performa
CREATE INDEX idx_rate_limits_identifier ON rate_limits(identifier, identifier_type);
CREATE INDEX idx_rate_limits_action ON rate_limits(action);
CREATE INDEX idx_rate_limits_window ON rate_limits(window_end);