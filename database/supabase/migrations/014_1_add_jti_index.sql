-- PPL/database/supabase/migrations/014_1_add_jti_index.sql
-- Add index untuk access_token_jti untuk performance optimization
-- JTI digunakan untuk:
-- 1. Token revocation - bisa invalidate specific access token
-- 2. Prevent replay attacks
-- 3. Audit trail

-- Index untuk JTI lookup (untuk validasi dan revocation)
CREATE INDEX IF NOT EXISTS idx_user_sessions_access_token_jti
ON user_sessions(access_token_jti)
WHERE access_token_jti IS NOT NULL;

-- Comment untuk dokumentasi
COMMENT ON COLUMN user_sessions.access_token_jti IS 'JWT ID dari access token untuk tracking dan revocation. Diupdate setiap kali access token di-refresh.';
