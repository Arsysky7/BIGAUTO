-- PPL/database/supabase/migrations/016_create_login_otps.sql
-- Table: login_otps
-- Deskripsi: OTP untuk login

CREATE TABLE login_otps (
    id SERIAL PRIMARY KEY,
    user_id INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    
    -- OTP
    otp_code VARCHAR(6) NOT NULL,
    otp_hash VARCHAR(255) NOT NULL,
    
    -- Validity
    expires_at TIMESTAMPTZ NOT NULL,
    is_used BOOLEAN DEFAULT false,
    used_at TIMESTAMPTZ,
    
    -- Rate limiting
    attempt_count INT DEFAULT 0,
    blocked_until TIMESTAMPTZ,
    
    -- Device info
    ip_address INET,
    user_agent TEXT,
    
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Index untuk performa
CREATE INDEX idx_login_otps_user_id ON login_otps(user_id);
CREATE INDEX idx_login_otps_otp_hash ON login_otps(otp_hash);
CREATE INDEX idx_login_otps_expires_at ON login_otps(expires_at);