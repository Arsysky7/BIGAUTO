-- PPL/database/supabase/migrations/017_update_users_table.sql

-- Tambahan kolom untuk users table

ALTER TABLE users ADD COLUMN IF NOT EXISTS email_verified BOOLEAN DEFAULT false;
ALTER TABLE users ADD COLUMN IF NOT EXISTS email_verified_at TIMESTAMPTZ;
ALTER TABLE users ADD COLUMN IF NOT EXISTS last_login_at TIMESTAMPTZ;
ALTER TABLE users ADD COLUMN IF NOT EXISTS login_count INT DEFAULT 0;
ALTER TABLE users ADD COLUMN IF NOT EXISTS is_active BOOLEAN DEFAULT true;
ALTER TABLE users ADD COLUMN IF NOT EXISTS deactivated_at TIMESTAMPTZ;

-- OTP rate limiting
ALTER TABLE users ADD COLUMN IF NOT EXISTS otp_request_count INT DEFAULT 0;
ALTER TABLE users ADD COLUMN IF NOT EXISTS otp_blocked_until TIMESTAMPTZ;
ALTER TABLE users ADD COLUMN IF NOT EXISTS last_otp_request_at TIMESTAMPTZ;