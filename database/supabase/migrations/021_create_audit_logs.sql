-- // PPL/database/supabase/migrations/021_create_audit_logs.sql  
-- Table: audit_logs
-- Deskripsi: Audit trail semua aktivitas penting

CREATE TABLE audit_logs (
    id SERIAL PRIMARY KEY,
    
    -- Actor
    user_id INT REFERENCES users(id) ON DELETE SET NULL,
    ip_address INET,
    user_agent TEXT,
    
    -- Action
    action VARCHAR(100) NOT NULL,
    entity_type VARCHAR(50),
    entity_id INT,
    
    -- Changes
    old_values JSONB,
    new_values JSONB,
    
    -- Context
    request_id VARCHAR(100),
    service_name VARCHAR(50),
    endpoint VARCHAR(255),
    http_method VARCHAR(10),
    
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Index untuk performa
CREATE INDEX idx_audit_logs_user_id ON audit_logs(user_id);
CREATE INDEX idx_audit_logs_action ON audit_logs(action);
CREATE INDEX idx_audit_logs_entity ON audit_logs(entity_type, entity_id);
CREATE INDEX idx_audit_logs_created_at ON audit_logs(created_at DESC);