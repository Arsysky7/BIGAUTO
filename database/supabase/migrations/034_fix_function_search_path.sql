-- Fix Function Search Path Mutable warnings
-- Tambahkan SET search_path ke semua functions

-- ============================================
-- FIX TRIGGERS FUNCTIONS
-- ============================================

-- Fix: update_vehicle_status_on_sale
CREATE OR REPLACE FUNCTION update_vehicle_status_on_sale()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.status = 'paid' AND OLD.status != 'paid' THEN
        UPDATE vehicles SET status = 'pending_sale', updated_at = NOW() WHERE id = NEW.vehicle_id;
    ELSIF NEW.status = 'completed' AND OLD.status != 'completed' THEN
        UPDATE vehicles SET status = 'sold', updated_at = NOW() WHERE id = NEW.vehicle_id;
    ELSIF (NEW.status IN ('cancelled', 'rejected')) AND (OLD.status NOT IN ('cancelled', 'rejected')) THEN
        UPDATE vehicles SET status = 'available', updated_at = NOW() WHERE id = NEW.vehicle_id;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER SET search_path = public, pg_temp;

-- Fix: create_sale_transaction_on_payment
CREATE OR REPLACE FUNCTION create_sale_transaction_on_payment()
RETURNS TRIGGER AS $$
DECLARE
    v_sale_order sale_orders%ROWTYPE;
    v_commission_rate DECIMAL(5, 2);
    v_commission_amount DECIMAL(12, 2);
    v_net_amount DECIMAL(12, 2);
BEGIN
    IF NEW.status = 'success' AND OLD.status != 'success' THEN
        SELECT * INTO v_sale_order FROM sale_orders WHERE id = NEW.booking_id;

        IF FOUND AND NEW.booking_type = 'sale' THEN
            SELECT commission_percentage INTO v_commission_rate
            FROM commission_settings
            WHERE transaction_type = 'sale' AND is_active = true
            ORDER BY effective_from DESC LIMIT 1;

            v_commission_rate := COALESCE(v_commission_rate, 5.00);
            v_commission_amount := (NEW.amount * v_commission_rate / 100);
            v_net_amount := NEW.amount - v_commission_amount;

            INSERT INTO transaction_logs (
                seller_id, transaction_type, amount, commission_amount,
                net_amount, commission_rate, reference_type, reference_id,
                payment_id, description
            ) VALUES (
                v_sale_order.seller_id, 'sale_payment', NEW.amount,
                v_commission_amount, v_net_amount, v_commission_rate,
                'sale_order', v_sale_order.id, NEW.id,
                'Payment received for sale order ' || v_sale_order.order_id
            );
        END IF;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER SET search_path = public, pg_temp;

-- Fix: update_seller_balance_on_completion
CREATE OR REPLACE FUNCTION update_seller_balance_on_completion()
RETURNS TRIGGER AS $$
DECLARE
    v_net_amount DECIMAL(12, 2);
BEGIN
    IF NEW.status = 'completed' AND OLD.status != 'completed' THEN
        SELECT net_amount INTO v_net_amount
        FROM transaction_logs
        WHERE reference_type = 'sale_order' AND reference_id = NEW.id
        LIMIT 1;

        IF FOUND THEN
            INSERT INTO seller_balance (seller_id, available_balance, pending_balance)
            VALUES (NEW.seller_id, v_net_amount, 0)
            ON CONFLICT (seller_id) DO UPDATE
            SET available_balance = seller_balance.available_balance + v_net_amount,
                updated_at = NOW();
        END IF;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER SET search_path = public, pg_temp;

-- Fix: update_vehicle_rating
CREATE OR REPLACE FUNCTION update_vehicle_rating()
RETURNS TRIGGER AS $$
DECLARE
    v_avg_rating DECIMAL(3, 2);
    v_review_count INT;
BEGIN
    SELECT AVG(rating), COUNT(*) INTO v_avg_rating, v_review_count
    FROM reviews
    WHERE reviewable_type = 'vehicle' AND reviewable_id = NEW.reviewable_id;

    UPDATE vehicles
    SET rating = v_avg_rating, review_count = v_review_count, updated_at = NOW()
    WHERE id = NEW.reviewable_id;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER SET search_path = public, pg_temp;

-- ============================================
-- FIX HELPER FUNCTIONS
-- ============================================

-- Fix: calculate_commission
CREATE OR REPLACE FUNCTION calculate_commission(
    p_transaction_type VARCHAR(20),
    p_amount DECIMAL(12, 2)
)
RETURNS TABLE(
    commission_rate DECIMAL(5, 2),
    commission_amount DECIMAL(12, 2),
    net_amount DECIMAL(12, 2)
) AS $$
DECLARE
    v_rate DECIMAL(5, 2);
    v_min_commission DECIMAL(12, 2);
    v_max_commission DECIMAL(12, 2);
    v_commission DECIMAL(12, 2);
BEGIN
    SELECT cs.commission_percentage, cs.min_commission, cs.max_commission
    INTO v_rate, v_min_commission, v_max_commission
    FROM commission_settings cs
    WHERE cs.transaction_type = p_transaction_type
      AND cs.is_active = true
      AND cs.effective_from <= NOW()
      AND (cs.effective_until IS NULL OR cs.effective_until >= NOW())
    ORDER BY cs.effective_from DESC LIMIT 1;

    IF v_rate IS NULL THEN
        v_rate := 5.00;
    END IF;

    v_commission := (p_amount * v_rate / 100);

    IF v_min_commission IS NOT NULL AND v_commission < v_min_commission THEN
        v_commission := v_min_commission;
    END IF;

    IF v_max_commission IS NOT NULL AND v_commission > v_max_commission THEN
        v_commission := v_max_commission;
    END IF;

    RETURN QUERY SELECT v_rate, v_commission, (p_amount - v_commission);
END;
$$ LANGUAGE plpgsql STABLE SECURITY DEFINER SET search_path = public, pg_temp;

-- Fix: check_vehicle_availability
CREATE OR REPLACE FUNCTION check_vehicle_availability(
    p_vehicle_id INT,
    p_start_date TIMESTAMPTZ,
    p_end_date TIMESTAMPTZ
)
RETURNS BOOLEAN AS $$
DECLARE
    v_is_available BOOLEAN;
BEGIN
    SELECT NOT EXISTS (
        SELECT 1 FROM rental_bookings
        WHERE vehicle_id = p_vehicle_id
          AND status NOT IN ('cancelled', 'rejected')
          AND (
            (pickup_datetime, return_datetime) OVERLAPS (p_start_date, p_end_date)
          )
    ) INTO v_is_available;

    RETURN v_is_available;
END;
$$ LANGUAGE plpgsql STABLE SECURITY DEFINER SET search_path = public, pg_temp;

-- Fix: get_seller_total_sales
CREATE OR REPLACE FUNCTION get_seller_total_sales(p_seller_id INT)
RETURNS DECIMAL(12, 2) AS $$
DECLARE
    v_total DECIMAL(12, 2);
BEGIN
    SELECT COALESCE(SUM(final_price), 0) INTO v_total
    FROM sale_orders
    WHERE seller_id = p_seller_id AND status = 'completed';

    RETURN v_total;
END;
$$ LANGUAGE plpgsql STABLE SECURITY DEFINER SET search_path = public, pg_temp;

-- Fix: get_seller_total_rentals
CREATE OR REPLACE FUNCTION get_seller_total_rentals(p_seller_id INT)
RETURNS DECIMAL(12, 2) AS $$
DECLARE
    v_total DECIMAL(12, 2);
BEGIN
    SELECT COALESCE(SUM(p.amount), 0) INTO v_total
    FROM payments p
    JOIN rental_bookings rb ON rb.id = p.booking_id
    JOIN vehicles v ON v.id = rb.vehicle_id
    WHERE v.seller_id = p_seller_id
      AND p.status = 'success'
      AND p.booking_type = 'rental';

    RETURN v_total;
END;
$$ LANGUAGE plpgsql STABLE SECURITY DEFINER SET search_path = public, pg_temp;

-- Fix: generate_order_id
CREATE OR REPLACE FUNCTION generate_order_id(p_prefix VARCHAR(10))
RETURNS VARCHAR(50) AS $$
DECLARE
    v_timestamp VARCHAR(20);
    v_random VARCHAR(10);
BEGIN
    v_timestamp := TO_CHAR(NOW(), 'YYYYMMDD-HH24MISS');
    v_random := LPAD(FLOOR(RANDOM() * 99999)::TEXT, 5, '0');

    RETURN p_prefix || '-' || v_timestamp || '-' || v_random;
END;
$$ LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path = public, pg_temp;

-- ============================================
-- FIX AUTO-UPDATE TIMESTAMPS
-- ============================================

-- Fix: update_updated_at_column
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER SET search_path = public, pg_temp;

-- Recreate triggers untuk updated_at (jika belum ada)
DROP TRIGGER IF EXISTS set_updated_at ON users;
CREATE TRIGGER set_updated_at BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

DROP TRIGGER IF EXISTS set_updated_at ON vehicles;
CREATE TRIGGER set_updated_at BEFORE UPDATE ON vehicles
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

DROP TRIGGER IF EXISTS set_updated_at ON rental_bookings;
CREATE TRIGGER set_updated_at BEFORE UPDATE ON rental_bookings
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

DROP TRIGGER IF EXISTS set_updated_at ON sale_orders;
CREATE TRIGGER set_updated_at BEFORE UPDATE ON sale_orders
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
