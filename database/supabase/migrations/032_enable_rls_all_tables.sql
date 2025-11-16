-- Enable Row Level Security (RLS) untuk semua tables
-- CRITICAL: Tanpa RLS = security hole!

-- User & Auth
ALTER TABLE users ENABLE ROW LEVEL SECURITY;
ALTER TABLE email_verifications ENABLE ROW LEVEL SECURITY;
ALTER TABLE login_otps ENABLE ROW LEVEL SECURITY;
ALTER TABLE user_sessions ENABLE ROW LEVEL SECURITY;

-- Vehicles
ALTER TABLE vehicles ENABLE ROW LEVEL SECURITY;
ALTER TABLE vehicle_brands ENABLE ROW LEVEL SECURITY;
ALTER TABLE vehicle_models ENABLE ROW LEVEL SECURITY;

-- Bookings & Orders
ALTER TABLE rental_bookings ENABLE ROW LEVEL SECURITY;
ALTER TABLE testdrive_bookings ENABLE ROW LEVEL SECURITY;
ALTER TABLE sale_orders ENABLE ROW LEVEL SECURITY;

-- Payments & Finance
ALTER TABLE payments ENABLE ROW LEVEL SECURITY;
ALTER TABLE seller_balance ENABLE ROW LEVEL SECURITY;
ALTER TABLE withdrawals ENABLE ROW LEVEL SECURITY;
ALTER TABLE transaction_logs ENABLE ROW LEVEL SECURITY;
ALTER TABLE commission_settings ENABLE ROW LEVEL SECURITY;

-- Social
ALTER TABLE reviews ENABLE ROW LEVEL SECURITY;
ALTER TABLE favorites ENABLE ROW LEVEL SECURITY;
ALTER TABLE conversations ENABLE ROW LEVEL SECURITY;
ALTER TABLE messages ENABLE ROW LEVEL SECURITY;
ALTER TABLE notifications ENABLE ROW LEVEL SECURITY;

-- Master & System
ALTER TABLE cities ENABLE ROW LEVEL SECURITY;
ALTER TABLE audit_logs ENABLE ROW LEVEL SECURITY;
ALTER TABLE rate_limits ENABLE ROW LEVEL SECURITY;

-- Helper: Get user ID dari JWT (public schema karena no permission ke auth schema)
CREATE OR REPLACE FUNCTION current_user_id()
RETURNS INTEGER AS $$
  SELECT NULLIF(current_setting('request.jwt.claims', true)::json->>'sub', '')::INTEGER;
$$ LANGUAGE sql STABLE SECURITY DEFINER SET search_path = public, pg_temp;

-- Helper: Cek apakah user adalah seller
CREATE OR REPLACE FUNCTION current_user_is_seller()
RETURNS BOOLEAN AS $$
  SELECT EXISTS (
    SELECT 1 FROM users WHERE id = current_user_id() AND is_seller = true
  );
$$ LANGUAGE sql STABLE SECURITY DEFINER SET search_path = public, pg_temp;
