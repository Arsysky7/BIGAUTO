-- ============================================================================
-- BIG AUTO - COMPLETE DATABASE SCHEMA
-- ============================================================================

-- ============================================================================

-- ============================================================================
-- SECTION 1: EXTENSIONS & CONFIGURATION
-- ============================================================================

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "pgcrypto";      -- Encryption functions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";     -- UUID generation
CREATE EXTENSION IF NOT EXISTS "pg_trgm";       -- Text search (trigram)

-- Set timezone to Indonesia
SET timezone = 'Asia/Jakarta';

-- ============================================================================
-- SECTION 2: ENUMS & TYPES
-- ============================================================================

-- User role types
CREATE TYPE user_role_enum AS ENUM ('customer', 'seller', 'guest');

-- Transaction types untuk financial logs
CREATE TYPE transaction_type_enum AS ENUM (
    'rental_payment', 'rental_refund',
    'sale_payment', 'seller_credit',
    'seller_withdrawal', 'commission_deduction'
);

-- Payment status
CREATE TYPE payment_status_enum AS ENUM (
    'pending', 'success', 'failed', 'expired', 'refunded'
);

-- Booking types
CREATE TYPE booking_category_enum AS ENUM ('rental', 'sale');

-- Review types
CREATE TYPE review_category_enum AS ENUM ('rental', 'sale');

-- ============================================================================
-- SECTION 3: MASTER DATA TABLES
-- ============================================================================

-- Master data: Kota-kota di Indonesia
CREATE TABLE cities (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL UNIQUE,
    province VARCHAR(100) NOT NULL,
    is_major_city BOOLEAN DEFAULT false,
    latitude NUMERIC,
    longitude NUMERIC,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Index untuk quick city lookup
CREATE INDEX idx_cities_name ON cities(name);
CREATE INDEX idx_cities_major ON cities(is_major_city) WHERE is_major_city = true;

-- Master data: Merk mobil
CREATE TABLE vehicle_brands (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL UNIQUE,
    logo_url TEXT,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Master data: Model mobil
CREATE TABLE vehicle_models (
    id SERIAL PRIMARY KEY,
    brand_id INTEGER NOT NULL REFERENCES vehicle_brands(id) ON DELETE RESTRICT,
    name VARCHAR(100) NOT NULL,
    vehicle_type VARCHAR(50),
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(brand_id, name)
);

-- ============================================================================
-- SECTION 4: USERS & AUTHENTICATION
-- ============================================================================

-- Users table (customer & seller)
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    name VARCHAR(100) NOT NULL,
    phone VARCHAR(20) NOT NULL,

    -- Email verification
    email_verified BOOLEAN DEFAULT false,
    email_verified_at TIMESTAMPTZ,

    -- Account status
    is_active BOOLEAN DEFAULT true,
    deactivated_at TIMESTAMPTZ,

    -- OTP rate limiting
    otp_request_count INTEGER DEFAULT 0,
    otp_blocked_until TIMESTAMPTZ,
    last_otp_request_at TIMESTAMPTZ,

    -- User roles (hybrid: bisa jadi customer & seller sekaligus)
    is_seller BOOLEAN DEFAULT false,

    -- Profile data
    address TEXT,
    city VARCHAR(100),
    profile_photo TEXT,
    business_name VARCHAR(100),

    -- Tracking
    last_login_at TIMESTAMPTZ,
    login_count INTEGER DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Index untuk user lookup
CREATE INDEX idx_users_email ON users(email) WHERE is_active = true;
CREATE INDEX idx_users_seller ON users(is_seller) WHERE is_seller = true;
CREATE INDEX idx_users_phone ON users(phone);

-- Email verification tokens
CREATE TABLE email_verifications (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token VARCHAR(255) NOT NULL UNIQUE,
    email VARCHAR(255) NOT NULL,
    is_used BOOLEAN DEFAULT false,
    expires_at TIMESTAMPTZ NOT NULL,
    verified_at TIMESTAMPTZ,
    sent_count INTEGER DEFAULT 1,
    last_sent_at TIMESTAMPTZ DEFAULT NOW(),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_email_verifications_token ON email_verifications(token);
CREATE INDEX idx_email_verifications_user ON email_verifications(user_id);

-- OTP untuk login
CREATE TABLE login_otps (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    otp_code VARCHAR(10) NOT NULL,
    otp_hash VARCHAR(255) NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    is_used BOOLEAN DEFAULT false,
    used_at TIMESTAMPTZ,
    attempt_count INTEGER DEFAULT 0,
    blocked_until TIMESTAMPTZ,
    ip_address INET,
    user_agent TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_login_otps_user ON login_otps(user_id);
CREATE INDEX idx_login_otps_code ON login_otps(otp_hash);

-- User sessions (refresh tokens)
CREATE TABLE user_sessions (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    refresh_token VARCHAR(500) NOT NULL UNIQUE,
    access_token_jti VARCHAR(255),
    user_agent TEXT,
    ip_address INET,
    device_name VARCHAR(100),
    expires_at TIMESTAMPTZ NOT NULL,
    last_activity TIMESTAMPTZ DEFAULT NOW(),
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_sessions_user ON user_sessions(user_id);
CREATE INDEX idx_sessions_token ON user_sessions(refresh_token);
CREATE INDEX idx_sessions_active ON user_sessions(is_active) WHERE is_active = true;

-- ============================================================================
-- SECTION 5: SECURITY TABLES (SECURITY_RULES.md)
-- ============================================================================

-- JWT Blacklist - Hanya bisa diakses lewat fungsi aman
CREATE TABLE jwt_blacklist (
    id SERIAL PRIMARY KEY,
    token_jti VARCHAR(255) NOT NULL UNIQUE,
    token_type VARCHAR(20) NOT NULL CHECK (token_type IN ('access', 'refresh')),
    blacklisted_at TIMESTAMPTZ DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    reason VARCHAR(50) NOT NULL CHECK (
        reason IN (
            'user_logout', 'password_change', 'account_suspension',
            'security_violation', 'token_compromise', 'session_timeout',
            'forced_logout', 'refresh_revocation'
        )
    ),
    detailed_reason TEXT,
    user_id INTEGER REFERENCES users(id) ON DELETE SET NULL,
    ip_address INET,
    user_agent TEXT,
    session_id VARCHAR(255),
    blacklisted_by VARCHAR(50) DEFAULT 'system',
    is_permanent BOOLEAN DEFAULT false,
    is_active BOOLEAN DEFAULT true,
    auto_cleanup BOOLEAN DEFAULT false,
    cleanup_scheduled_at TIMESTAMPTZ,
    last_accessed_at TIMESTAMPTZ DEFAULT NOW()
);

-- Index untuk blacklist check yang cepat
CREATE INDEX idx_jwt_blacklist_lookup ON jwt_blacklist(token_jti, token_type)
    WHERE is_active = true;
CREATE INDEX idx_jwt_blacklist_user ON jwt_blacklist(user_id);
CREATE INDEX idx_jwt_blacklist_cleanup ON jwt_blacklist(auto_cleanup)
    WHERE auto_cleanup = true;

-- Security incidents tracking
CREATE TABLE security_incidents (
    id SERIAL PRIMARY KEY,
    incident_id VARCHAR(50) UNIQUE DEFAULT gen_random_uuid()::VARCHAR(50),
    incident_type VARCHAR(50) NOT NULL CHECK (
        incident_type IN (
            'csrf_attack', 'rate_limit_violation', 'brute_force_attempt',
            'suspicious_pattern', 'data_exfiltration', 'privilege_escalation',
            'sql_injection_attempt', 'xss_attempt', 'unauthorized_access'
        )
    ),
    severity VARCHAR(20) NOT NULL CHECK (severity IN ('low', 'medium', 'high', 'critical')),
    priority INTEGER DEFAULT 1 CHECK (priority BETWEEN 1 AND 5),
    user_id INTEGER REFERENCES users(id) ON DELETE SET NULL,
    session_id VARCHAR(255),
    ip_address INET,
    user_agent TEXT,
    description TEXT NOT NULL,
    endpoint VARCHAR(255),
    http_method VARCHAR(10),
    request_payload JSONB,
    response_data JSONB,
    status VARCHAR(50) DEFAULT 'open' CHECK (
        status IN ('open', 'investigating', 'resolved', 'false_positive')
    ),
    resolved_at TIMESTAMPTZ,
    resolution_notes TEXT,
    false_positive_reason TEXT,
    assigned_to VARCHAR(100),
    tags TEXT[],
    metadata JSONB,
    auto_detected BOOLEAN DEFAULT false,
    auto_resolved BOOLEAN DEFAULT false,
    requires_manual_review BOOLEAN DEFAULT false,
    detected_at TIMESTAMPTZ DEFAULT NOW(),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    incident_number INTEGER DEFAULT nextval('security_incident_seq')
);

-- Index untuk security monitoring
CREATE INDEX idx_security_incidents_type ON security_incidents(incident_type);
CREATE INDEX idx_security_incidents_severity ON security_incidents(severity);
CREATE INDEX idx_security_incidents_status ON security_incidents(status);
CREATE INDEX idx_security_incidents_user ON security_incidents(user_id);

-- ============================================================================
-- SECTION 6: SECURITY FUNCTIONS 
-- ============================================================================

-- Secure blacklist validation function
CREATE OR REPLACE FUNCTION is_token_blacklisted_v2(
    p_token_jti TEXT,
    p_token_type TEXT DEFAULT 'access'
)
RETURNS BOOLEAN
LANGUAGE sql
STABLE SECURITY DEFINER
SET search_path TO public
AS $$
SELECT EXISTS (
    SELECT 1 FROM jwt_blacklist
    WHERE token_jti = p_token_jti
    AND token_type = p_token_type
    AND is_active = true
    AND (expires_at > NOW() OR is_permanent = true)
);
$$;

-- Helper: Revoke semua token user (logout, password change, dll)
CREATE OR REPLACE FUNCTION revoke_user_tokens(
    p_user_id INTEGER,
    p_reason VARCHAR(50),
    p_detailed_reason TEXT DEFAULT NULL
)
RETURNS INTEGER
LANGUAGE plpgsql
SECURITY DEFINER
AS $$
DECLARE
    v_count INTEGER;
BEGIN
    INSERT INTO jwt_blacklist (
        token_jti,
        token_type,
        reason,
        detailed_reason,
        user_id,
        is_permanent
    )
    SELECT
        access_token_jti,
        'access',
        p_reason,
        p_detailed_reason,
        p_user_id,
        true
    FROM user_sessions
    WHERE user_id = p_user_id AND is_active = true
    AND access_token_jti IS NOT NULL;

    GET DIAGNOSTICS v_count = ROW_COUNT;

    -- Deactivate sessions
    UPDATE user_sessions
    SET is_active = false
    WHERE user_id = p_user_id;

    RETURN v_count;
END;
$$;

-- ============================================================================
-- SECTION 7: VEHICLES
-- ============================================================================

CREATE TABLE vehicles (
    id SERIAL PRIMARY KEY,
    seller_id INTEGER NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    title VARCHAR(255) NOT NULL,
    category VARCHAR(20) NOT NULL CHECK (category IN ('rental', 'sale')),
    price NUMERIC(15, 2) NOT NULL,
    brand VARCHAR(100) NOT NULL,
    model VARCHAR(100) NOT NULL,
    year INTEGER NOT NULL,
    transmission VARCHAR(20) CHECK (transmission IN ('manual', 'automatic')),
    fuel_type VARCHAR(20) CHECK (fuel_type IN ('bensin', 'diesel', 'hybrid', 'electric')),
    engine_capacity INTEGER,
    mileage INTEGER,
    seats INTEGER NOT NULL,
    doors INTEGER,
    luggage_capacity INTEGER,
    vehicle_type VARCHAR(50) NOT NULL,
    is_luxury BOOLEAN DEFAULT false,
    is_flood_free BOOLEAN DEFAULT false,
    tax_active BOOLEAN DEFAULT false,
    has_bpkb BOOLEAN DEFAULT false,
    has_stnk BOOLEAN DEFAULT false,
    description TEXT,
    rental_terms TEXT,
    city VARCHAR(100) NOT NULL,
    address TEXT NOT NULL,
    latitude NUMERIC,
    longitude NUMERIC,
    area_coverage JSONB,
    photos JSONB NOT NULL,
    status VARCHAR(20) DEFAULT 'available' CHECK (
        status IN ('available', 'pending_sale', 'sold')
    ),
    rating NUMERIC(3, 2) DEFAULT 0.0,
    review_count INTEGER DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Index untuk vehicle search & filter
CREATE INDEX idx_vehicles_seller ON vehicles(seller_id);
CREATE INDEX idx_vehicles_category ON vehicles(category);
CREATE INDEX idx_vehicles_city ON vehicles(city);
CREATE INDEX idx_vehicles_status ON vehicles(status);
CREATE INDEX idx_vehicles_price ON vehicles(price);
CREATE INDEX idx_vehicles_luxury ON vehicles(is_luxury) WHERE is_luxury = true;
CREATE INDEX idx_vehicles_brand_model ON vehicles(brand, model);
CREATE INDEX idx_vehicles_year ON vehicles(year);
CREATE INDEX idx_vehicles_rating ON vehicles(rating DESC);

-- Full text search index
CREATE INDEX idx_vehicles_search ON vehicles USING gin(
    to_tsvector('english',
        COALESCE(title, '') || ' ' ||
        COALESCE(brand, '') || ' ' ||
        COALESCE(model, '') || ' ' ||
        COALESCE(description, '')
    )
);

-- ============================================================================
-- SECTION 8: RENTAL BOOKINGS
-- ============================================================================

CREATE TABLE rental_bookings (
    id SERIAL PRIMARY KEY,
    vehicle_id INTEGER NOT NULL REFERENCES vehicles(id) ON DELETE RESTRICT,
    customer_id INTEGER NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    seller_id INTEGER NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    order_id VARCHAR(50) NOT NULL UNIQUE,
    pickup_date TIMESTAMPTZ NOT NULL,
    return_date TIMESTAMPTZ NOT NULL,
    actual_pickup_at TIMESTAMPTZ,
    actual_return_at TIMESTAMPTZ,
    customer_name VARCHAR(100) NOT NULL,
    customer_phone VARCHAR(20) NOT NULL,
    customer_email VARCHAR(255) NOT NULL,
    ktp_photo TEXT,
    total_days INTEGER NOT NULL,
    price_per_day NUMERIC(15, 2) NOT NULL,
    total_price NUMERIC(15, 2) NOT NULL,
    notes TEXT,
    status VARCHAR(50) DEFAULT 'pending_payment' CHECK (
        status IN (
            'pending_payment', 'paid', 'akan_datang',
            'berjalan', 'selesai', 'cancelled'
        )
    ),
    cancel_reason TEXT,
    cancelled_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT rental_dates_valid CHECK (return_date > pickup_date)
);

-- Index untuk rental queries
CREATE INDEX idx_rental_vehicle ON rental_bookings(vehicle_id);
CREATE INDEX idx_rental_customer ON rental_bookings(customer_id);
CREATE INDEX idx_rental_seller ON rental_bookings(seller_id);
CREATE INDEX idx_rental_status ON rental_bookings(status);
CREATE INDEX idx_rental_dates ON rental_bookings(pickup_date, return_date);

-- ============================================================================
-- SECTION 9: TEST DRIVE BOOKINGS
-- ============================================================================

CREATE TABLE testdrive_bookings (
    id SERIAL PRIMARY KEY,
    vehicle_id INTEGER NOT NULL REFERENCES vehicles(id) ON DELETE RESTRICT,
    customer_id INTEGER NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    seller_id INTEGER NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    requested_date TIMESTAMPTZ NOT NULL,
    requested_time VARCHAR(50) NOT NULL,
    reschedule_slots JSONB,
    customer_name VARCHAR(100) NOT NULL,
    customer_phone VARCHAR(20) NOT NULL,
    customer_email VARCHAR(255) NOT NULL,
    notes TEXT,
    status VARCHAR(50) DEFAULT 'menunggu_konfirmasi' CHECK (
        status IN (
            'menunggu_konfirmasi', 'seller_reschedule',
            'diterima', 'selesai', 'cancelled', 'timeout'
        )
    ),
    timeout_at TIMESTAMPTZ,
    cancel_reason TEXT,
    cancelled_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_testdrive_vehicle ON testdrive_bookings(vehicle_id);
CREATE INDEX idx_testdrive_customer ON testdrive_bookings(customer_id);
CREATE INDEX idx_testdrive_seller ON testdrive_bookings(seller_id);
CREATE INDEX idx_testdrive_status ON testdrive_bookings(status);

-- ============================================================================
-- SECTION 10: SALE ORDERS (JUAL BELI)
-- ============================================================================

CREATE TABLE sale_orders (
    id SERIAL PRIMARY KEY,
    vehicle_id INTEGER NOT NULL REFERENCES vehicles(id) ON DELETE RESTRICT,
    buyer_id INTEGER NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    seller_id INTEGER NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    testdrive_booking_id INTEGER REFERENCES testdrive_bookings(id) ON DELETE SET NULL,
    order_id VARCHAR(50) NOT NULL UNIQUE,
    asking_price NUMERIC(15, 2) NOT NULL,
    offer_price NUMERIC(15, 2),
    counter_offer_price NUMERIC(15, 2),
    final_price NUMERIC(15, 2) NOT NULL,
    buyer_name VARCHAR(100) NOT NULL,
    buyer_phone VARCHAR(20) NOT NULL,
    buyer_email VARCHAR(255) NOT NULL,
    buyer_address TEXT,
    buyer_ktp_photo TEXT,
    status VARCHAR(50) DEFAULT 'pending_confirmation' CHECK (
        status IN (
            'pending_confirmation', 'pending_payment', 'paid',
            'document_processing', 'completed', 'cancelled', 'rejected'
        )
    ),
    -- Document transfer tracking
    bpkb_transferred BOOLEAN DEFAULT false,
    stnk_transferred BOOLEAN DEFAULT false,
    faktur_transferred BOOLEAN DEFAULT false,
    pajak_transferred BOOLEAN DEFAULT false,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    confirmed_at TIMESTAMPTZ,
    paid_at TIMESTAMPTZ,
    document_transfer_started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    cancelled_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    cancel_reason TEXT,
    reject_reason TEXT,
    rejected_at TIMESTAMPTZ,
    buyer_notes TEXT,
    seller_notes TEXT
);

-- Index untuk sale order queries
CREATE INDEX idx_sale_vehicle ON sale_orders(vehicle_id);
CREATE INDEX idx_sale_buyer ON sale_orders(buyer_id);
CREATE INDEX idx_sale_seller ON sale_orders(seller_id);
CREATE INDEX idx_sale_status ON sale_orders(status);
CREATE INDEX idx_sale_testdrive ON sale_orders(testdrive_booking_id);

-- ============================================================================
-- SECTION 11: PAYMENTS (POLYMORPHIC)
-- ============================================================================

CREATE TABLE payments (
    id SERIAL PRIMARY KEY,
    -- Polymorphic: bisa reference rental atau sale
    rental_booking_id INTEGER REFERENCES rental_bookings(id) ON DELETE SET NULL,
    sale_order_id INTEGER REFERENCES sale_orders(id) ON DELETE SET NULL,
    order_id VARCHAR(50) NOT NULL UNIQUE,
    transaction_id VARCHAR(100),
    va_number VARCHAR(50),
    bank VARCHAR(50),
    payment_type VARCHAR(50),
    gross_amount NUMERIC(15, 2) NOT NULL,
    status VARCHAR(20) DEFAULT 'pending' CHECK (
        status IN ('pending', 'success', 'failed', 'expired', 'refunded')
    ),
    refund_amount NUMERIC(15, 2),
    refund_reason TEXT,
    paid_at TIMESTAMPTZ,
    expired_at TIMESTAMPTZ,
    refunded_at TIMESTAMPTZ,
    receipt_pdf_path TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    payment_for_type VARCHAR(20) CHECK (payment_for_type IN ('rental', 'sale'))
);

-- Constraint: must reference exactly one booking type
ALTER TABLE payments ADD CONSTRAINT payment_booking_check CHECK (
    (rental_booking_id IS NOT NULL)::INTEGER +
    (sale_order_id IS NOT NULL)::INTEGER = 1
);

CREATE INDEX idx_payment_rental ON payments(rental_booking_id);
CREATE INDEX idx_payment_sale ON payments(sale_order_id);
CREATE INDEX idx_payment_order ON payments(order_id);
CREATE INDEX idx_payment_status ON payments(status);

-- ============================================================================
-- SECTION 12: REVIEWS (POLYMORPHIC)
-- ============================================================================

CREATE TABLE reviews (
    id SERIAL PRIMARY KEY,
    -- Polymorphic: bisa reference rental atau sale
    rental_booking_id INTEGER REFERENCES rental_bookings(id) ON DELETE SET NULL,
    sale_order_id INTEGER REFERENCES sale_orders(id) ON DELETE SET NULL,
    vehicle_id INTEGER NOT NULL REFERENCES vehicles(id) ON DELETE RESTRICT,
    seller_id INTEGER NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    customer_id INTEGER NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    overall_rating INTEGER NOT NULL CHECK (overall_rating BETWEEN 1 AND 5),
    vehicle_condition_rating INTEGER CHECK (vehicle_condition_rating BETWEEN 1 AND 5),
    accuracy_rating INTEGER CHECK (accuracy_rating BETWEEN 1 AND 5),
    service_rating INTEGER CHECK (service_rating BETWEEN 1 AND 5),
    comment TEXT,
    photos JSONB,
    is_visible BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    review_for_type VARCHAR(20) CHECK (review_for_type IN ('rental', 'sale'))
);

ALTER TABLE reviews ADD CONSTRAINT review_booking_check CHECK (
    (rental_booking_id IS NOT NULL)::INTEGER +
    (sale_order_id IS NOT NULL)::INTEGER = 1
);

CREATE INDEX idx_review_vehicle ON reviews(vehicle_id);
CREATE INDEX idx_review_seller ON reviews(seller_id);
CREATE INDEX idx_review_customer ON reviews(customer_id);
CREATE INDEX idx_review_rental ON reviews(rental_booking_id);
CREATE INDEX idx_review_sale ON reviews(sale_order_id);

-- ============================================================================
-- SECTION 13: CHAT & MESSAGING
-- ============================================================================

CREATE TABLE conversations (
    id SERIAL PRIMARY KEY,
    customer_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    seller_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    vehicle_id INTEGER REFERENCES vehicles(id) ON DELETE SET NULL,
    last_message TEXT,
    last_message_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    -- Prevent duplicate conversations
    UNIQUE(customer_id, seller_id, vehicle_id)
);

CREATE INDEX idx_conversations_customer ON conversations(customer_id);
CREATE INDEX idx_conversations_seller ON conversations(seller_id);
CREATE INDEX idx_conversations_updated ON conversations(updated_at DESC);

CREATE TABLE messages (
    id SERIAL PRIMARY KEY,
    conversation_id INTEGER NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    sender_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    message_type VARCHAR(20) DEFAULT 'text' CHECK (message_type IN ('text', 'image')),
    media_url TEXT,
    thumbnail_url TEXT,
    is_read BOOLEAN DEFAULT false,
    read_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_messages_conversation ON messages(conversation_id, created_at DESC);
CREATE INDEX idx_messages_unread ON messages(conversation_id)
    WHERE is_read = false;

-- ============================================================================
-- SECTION 14: USER FAVORITES
-- ============================================================================

CREATE TABLE favorites (
    id SERIAL PRIMARY KEY,
    customer_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    vehicle_id INTEGER NOT NULL REFERENCES vehicles(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(customer_id, vehicle_id)
);

CREATE INDEX idx_favorites_customer ON favorites(customer_id);
CREATE INDEX idx_favorites_vehicle ON favorites(vehicle_id);

-- ============================================================================
-- SECTION 15: FINANCIAL & COMMISSION
-- ============================================================================

CREATE TABLE seller_balance (
    id SERIAL PRIMARY KEY,
    seller_id INTEGER NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    available_balance NUMERIC(15, 2) DEFAULT 0.0,
    pending_balance NUMERIC(15, 2) DEFAULT 0.0,
    total_earned NUMERIC(15, 2) DEFAULT 0.0,
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE withdrawals (
    id SERIAL PRIMARY KEY,
    seller_id INTEGER NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    amount NUMERIC(15, 2) NOT NULL,
    bank_name VARCHAR(100) NOT NULL,
    account_number VARCHAR(50) NOT NULL,
    account_holder_name VARCHAR(100) NOT NULL,
    status VARCHAR(20) DEFAULT 'pending' CHECK (
        status IN ('pending', 'processing', 'completed', 'failed')
    ),
    requested_at TIMESTAMPTZ DEFAULT NOW(),
    processed_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ
);

CREATE INDEX idx_withdrawals_seller ON withdrawals(seller_id);
CREATE INDEX idx_withdrawals_status ON withdrawals(status);

CREATE TABLE transaction_logs (
    id SERIAL PRIMARY KEY,
    transaction_type transaction_type_enum NOT NULL,
    user_id INTEGER REFERENCES users(id) ON DELETE SET NULL,
    booking_id INTEGER REFERENCES rental_bookings(id) ON DELETE SET NULL,
    payment_id INTEGER REFERENCES payments(id) ON DELETE SET NULL,
    withdrawal_id INTEGER REFERENCES withdrawals(id) ON DELETE SET NULL,
    sale_order_id INTEGER REFERENCES sale_orders(id) ON DELETE SET NULL,
    amount NUMERIC(15, 2) NOT NULL,
    commission_amount NUMERIC(15, 2),
    net_amount NUMERIC(15, 2),
    status VARCHAR(20) DEFAULT 'pending' CHECK (
        status IN ('pending', 'completed', 'failed', 'reversed')
    ),
    metadata JSONB,
    notes TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_transactions_user ON transaction_logs(user_id);
CREATE INDEX idx_transactions_type ON transaction_logs(transaction_type);

CREATE TABLE commission_settings (
    id SERIAL PRIMARY KEY,
    transaction_type VARCHAR(20) NOT NULL CHECK (transaction_type IN ('rental', 'sale')),
    commission_percentage NUMERIC(5, 2) NOT NULL CHECK (
        commission_percentage >= 0 AND commission_percentage <= 100
    ),
    min_commission NUMERIC(15, 2) CHECK (min_commission >= 0),
    max_commission NUMERIC(15, 2),
    effective_from TIMESTAMPTZ NOT NULL,
    effective_until TIMESTAMPTZ,
    is_active BOOLEAN DEFAULT true,
    description TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Default commission: 5%
INSERT INTO commission_settings (transaction_type, commission_percentage, effective_from)
VALUES
    ('rental', 5.00, NOW()),
    ('sale', 5.00, NOW());

-- ============================================================================
-- SECTION 16: NOTIFICATIONS
-- ============================================================================

CREATE TABLE notifications (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    type VARCHAR(50) NOT NULL,
    title VARCHAR(255) NOT NULL,
    message TEXT NOT NULL,
    related_id INTEGER,
    related_type VARCHAR(50),
    is_read BOOLEAN DEFAULT false,
    read_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_notifications_user ON notifications(user_id, is_read);
CREATE INDEX idx_notifications_type ON notifications(type);

-- ============================================================================
-- SECTION 17: RATE LIMITING & AUDIT
-- ============================================================================

CREATE TABLE rate_limits (
    id SERIAL PRIMARY KEY,
    identifier VARCHAR(255) NOT NULL,
    identifier_type VARCHAR(50) NOT NULL CHECK (
        identifier_type IN ('user', 'ip', 'email')
    ),
    action VARCHAR(100) NOT NULL,
    request_count INTEGER DEFAULT 1,
    window_start TIMESTAMPTZ DEFAULT NOW(),
    window_end TIMESTAMPTZ NOT NULL,
    is_blocked BOOLEAN DEFAULT false,
    blocked_until TIMESTAMPTZ,
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    service_name VARCHAR(100) DEFAULT 'unknown',
    endpoint VARCHAR(255) DEFAULT 'unknown',
    http_method VARCHAR(10) DEFAULT 'GET',
    request_size_mb NUMERIC(10, 2) DEFAULT 0,
    response_status INTEGER,
    blocked_reason TEXT
);

CREATE INDEX idx_ratelimits_lookup ON rate_limits(
    identifier, identifier_type, action, window_end
) WHERE is_blocked = false;

CREATE TABLE audit_logs (
    id SERIAL PRIMARY KEY,
    user_id INTEGER REFERENCES users(id) ON DELETE SET NULL,
    ip_address INET,
    user_agent TEXT,
    action VARCHAR(100) NOT NULL,
    entity_type VARCHAR(50),
    entity_id INTEGER,
    old_values JSONB,
    new_values JSONB,
    request_id VARCHAR(100),
    service_name VARCHAR(100),
    endpoint VARCHAR(255),
    http_method VARCHAR(10),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_audit_user ON audit_logs(user_id);
CREATE INDEX idx_audit_action ON audit_logs(action);
CREATE INDEX idx_audit_entity ON audit_logs(entity_type, entity_id);
CREATE INDEX idx_audit_created ON audit_logs(created_at DESC);

-- Security index usage stats (untuk monitoring)
CREATE TABLE security_index_usage_stats (
    id SERIAL PRIMARY KEY,
    index_name VARCHAR(255) NOT NULL,
    table_name VARCHAR(100) NOT NULL,
    query_count INTEGER DEFAULT 0,
    last_used TIMESTAMPTZ,
    avg_execution_time_ms NUMERIC,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(index_name, table_name)
);

-- ============================================================================
-- SECTION 18: TRIGGERS & FUNCTIONS
-- ============================================================================

-- Update timestamp trigger function
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Apply update_updated_at ke semua tables yang punya updated_at
CREATE TRIGGER trigger_users_updated_at BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trigger_vehicles_updated_at BEFORE UPDATE ON vehicles
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trigger_rental_updated_at BEFORE UPDATE ON rental_bookings
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trigger_testdrive_updated_at BEFORE UPDATE ON testdrive_bookings
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trigger_sale_updated_at BEFORE UPDATE ON sale_orders
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trigger_payments_updated_at BEFORE UPDATE ON payments
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trigger_reviews_updated_at BEFORE UPDATE ON reviews
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trigger_conversations_updated_at BEFORE UPDATE ON conversations
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trigger_seller_balance_updated_at BEFORE UPDATE ON seller_balance
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- Vehicle rating calculation trigger
CREATE OR REPLACE FUNCTION update_vehicle_rating()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE vehicles
    SET
        rating = (SELECT COALESCE(AVG(overall_rating), 0) FROM reviews WHERE vehicle_id = NEW.vehicle_id),
        review_count = (SELECT COUNT(*) FROM reviews WHERE vehicle_id = NEW.vehicle_id)
    WHERE id = NEW.vehicle_id;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_vehicle_rating_update AFTER INSERT OR UPDATE ON reviews
    FOR EACH ROW EXECUTE FUNCTION update_vehicle_rating();

-- ============================================================================
-- SECTION 9: ROW LEVEL SECURITY (RLS)
-- ============================================================================

-- Enable RLS on all tables
ALTER TABLE users ENABLE ROW LEVEL SECURITY;
ALTER TABLE vehicles ENABLE ROW LEVEL SECURITY;
ALTER TABLE rental_bookings ENABLE ROW LEVEL SECURITY;
ALTER TABLE testdrive_bookings ENABLE ROW LEVEL SECURITY;
ALTER TABLE sale_orders ENABLE ROW LEVEL SECURITY;
ALTER TABLE payments ENABLE ROW LEVEL SECURITY;
ALTER TABLE reviews ENABLE ROW LEVEL SECURITY;
ALTER TABLE conversations ENABLE ROW LEVEL SECURITY;
ALTER TABLE messages ENABLE ROW LEVEL SECURITY;
ALTER TABLE favorites ENABLE ROW LEVEL SECURITY;
ALTER TABLE seller_balance ENABLE ROW LEVEL SECURITY;
ALTER TABLE withdrawals ENABLE ROW LEVEL SECURITY;
ALTER TABLE notifications ENABLE ROW LEVEL SECURITY;

-- Drop existing policies jika ada
DROP POLICY IF EXISTS users_select_policy ON users;
DROP POLICY IF EXISTS users_update_policy ON users;
DROP POLICY IF EXISTS vehicles_select_policy ON vehicles;
DROP POLICY IF EXISTS vehicles_update_policy ON vehicles;

-- Users policies: user bisa lihat & update diri sendiri
CREATE POLICY users_select_policy ON users
    FOR SELECT
    USING (id = current_setting('app.current_user_id')::INTEGER);

CREATE POLICY users_update_policy ON users
    FOR UPDATE
    USING (id = current_setting('app.current_user_id')::INTEGER);

-- Vehicles policies: semua user bisa lihat, cuma seller yang punya bisa update
CREATE POLICY vehicles_select_policy ON vehicles
    FOR SELECT
    USING (true);

CREATE POLICY vehicles_update_policy ON vehicles
    FOR UPDATE
    USING (seller_id = current_setting('app.current_user_id')::INTEGER);

-- Notifications policies: user cuma bisa lihat notifnya sendiri
CREATE POLICY notifications_select_policy ON notifications
    FOR SELECT
    USING (user_id = current_setting('app.current_user_id')::INTEGER);

CREATE POLICY notifications_update_policy ON notifications
    FOR UPDATE
    USING (user_id = current_setting('app.current_user_id')::INTEGER);

-- Favorites policies: user cuma bisa lihat & manage favoritenya sendiri
CREATE POLICY favorites_select_policy ON favorites
    FOR SELECT
    USING (customer_id = current_setting('app.current_user_id')::INTEGER);

CREATE POLICY favorites_insert_policy ON favorites
    FOR INSERT
    WITH CHECK (customer_id = current_setting('app.current_user_id')::INTEGER);

CREATE POLICY favorites_delete_policy ON favorites
    FOR DELETE
    USING (customer_id = current_setting('app.current_user_id')::INTEGER);

-- ============================================================================
-- SECTION 20: HELPER FUNCTIONS
-- ============================================================================

-- Cek apakah user adalah participant dalam conversation
CREATE OR REPLACE FUNCTION is_conversation_participant(
    p_conversation_id INTEGER,
    p_user_id INTEGER
)
RETURNS BOOLEAN
LANGUAGE sql
STABLE
AS $$
SELECT EXISTS (
    SELECT 1 FROM conversations
    WHERE id = p_conversation_id
    AND (customer_id = p_user_id OR seller_id = p_user_id)
);
$$;

-- Hitung unread messages untuk conversation
CREATE OR REPLACE FUNCTION get_unread_message_count(
    p_conversation_id INTEGER,
    p_user_id INTEGER
)
RETURNS INTEGER
LANGUAGE sql
STABLE
AS $$
SELECT COALESCE(COUNT(*), 0)
FROM messages
WHERE conversation_id = p_conversation_id
AND sender_id != p_user_id
AND is_read = false;
$$;

-- Update conversation last message
CREATE OR REPLACE FUNCTION update_conversation_last_message()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE conversations
    SET
        last_message = NEW.content,
        last_message_at = NEW.created_at,
        updated_at = NOW()
    WHERE id = NEW.conversation_id;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_conversation_last_message
AFTER INSERT ON messages
FOR EACH ROW EXECUTE FUNCTION update_conversation_last_message();

-- ============================================================================
-- END OF SCHEMA
-- ============================================================================

-- Verification queries
DO $$
BEGIN
    RAISE NOTICE '===========================================';
    RAISE NOTICE 'BIG AUTO DATABASE SCHEMA LOADED SUCCESSFULLY';
    RAISE NOTICE 'Tables: %', (
        SELECT COUNT(*) FROM information_schema.tables
        WHERE table_schema = 'public'
        AND table_type = 'BASE TABLE'
    );
    RAISE NOTICE 'Security Functions: is_token_blacklisted_v2()';
    RAISE NOTICE 'RLS: Enabled';
    RAISE NOTICE '===========================================';
END $$;