-- PPL/database/supabase/migrations/001_create_users.sql

-- Table: users
-- Deskripsi: Data customer & seller (1 akun bisa keduanya)

CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    name VARCHAR(255) NOT NULL,
    phone VARCHAR(20) NOT NULL,

    -- Email verification (required untuk login)
    email_verified BOOLEAN DEFAULT false,
    email_verified_at TIMESTAMPTZ,

    -- Login tracking
    last_login_at TIMESTAMPTZ,
    login_count INTEGER DEFAULT 0,

    -- Account status
    is_active BOOLEAN DEFAULT true,
    deactivated_at TIMESTAMPTZ,

    -- OTP rate limiting (per user)
    otp_request_count INTEGER DEFAULT 0,
    otp_blocked_until TIMESTAMPTZ,
    last_otp_request_at TIMESTAMPTZ,

    -- Role: 1 akun bisa jadi customer sekaligus seller
    is_seller BOOLEAN DEFAULT false,
    
    -- Profile
    address TEXT,
    city VARCHAR(100),
    profile_photo VARCHAR(500),
    
    -- Seller info (jika is_seller = true)
    business_name VARCHAR(255),
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes untuk performance
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_email_verified ON users(email_verified);
CREATE INDEX idx_users_is_seller ON users(is_seller);
CREATE INDEX idx_users_is_active ON users(is_active);
CREATE INDEX idx_users_otp_blocked_until ON users(otp_blocked_until);