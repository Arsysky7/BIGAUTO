-- PPL/database/supabase/migrations/031_create_helper_functions.sql
-- Helper functions untuk business logic

-- ============================================
-- FUNCTION: Calculate commission for a transaction
-- ============================================

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
    -- Get current commission settings
    SELECT cs.commission_percentage, cs.min_commission, cs.max_commission
    INTO v_rate, v_min_commission, v_max_commission
    FROM commission_settings cs
    WHERE cs.transaction_type = p_transaction_type
      AND cs.is_active = true
      AND cs.effective_from <= NOW()
      AND (cs.effective_until IS NULL OR cs.effective_until >= NOW())
    ORDER BY cs.effective_from DESC
    LIMIT 1;

    -- Default to 5% if not found
    IF v_rate IS NULL THEN
        v_rate := 5.00;
    END IF;

    -- Calculate commission
    v_commission := (p_amount * v_rate / 100);

    -- Apply min/max limits if set
    IF v_min_commission IS NOT NULL AND v_commission < v_min_commission THEN
        v_commission := v_min_commission;
    END IF;

    IF v_max_commission IS NOT NULL AND v_commission > v_max_commission THEN
        v_commission := v_max_commission;
    END IF;

    -- Return result
    RETURN QUERY SELECT
        v_rate,
        v_commission,
        p_amount - v_commission;
END;
$$ LANGUAGE plpgsql;


-- ============================================
-- FUNCTION: Check if vehicle is available for sale
-- ============================================

CREATE OR REPLACE FUNCTION is_vehicle_available_for_sale(
    p_vehicle_id INT
)
RETURNS BOOLEAN AS $$
DECLARE
    v_vehicle vehicles%ROWTYPE;
    v_active_order_count INT;
BEGIN
    -- Get vehicle
    SELECT * INTO v_vehicle FROM vehicles WHERE id = p_vehicle_id;

    -- Check if vehicle exists and is for sale
    IF v_vehicle.id IS NULL OR v_vehicle.category != 'sale' THEN
        RETURN false;
    END IF;

    -- Check vehicle status
    IF v_vehicle.status != 'available' THEN
        RETURN false;
    END IF;

    -- Check if there are active orders
    SELECT COUNT(*) INTO v_active_order_count
    FROM sale_orders
    WHERE vehicle_id = p_vehicle_id
      AND status IN ('pending_confirmation', 'pending_payment', 'paid', 'document_processing');

    IF v_active_order_count > 0 THEN
        RETURN false;
    END IF;

    RETURN true;
END;
$$ LANGUAGE plpgsql;


-- ============================================
-- FUNCTION: Check if vehicle is available for rental
-- ============================================

CREATE OR REPLACE FUNCTION is_vehicle_available_for_rental(
    p_vehicle_id INT,
    p_pickup_date TIMESTAMPTZ,
    p_return_date TIMESTAMPTZ
)
RETURNS BOOLEAN AS $$
DECLARE
    v_vehicle vehicles%ROWTYPE;
    v_overlap_count INT;
BEGIN
    -- Get vehicle
    SELECT * INTO v_vehicle FROM vehicles WHERE id = p_vehicle_id;

    -- Check if vehicle exists and is for rental
    IF v_vehicle.id IS NULL OR v_vehicle.category != 'rental' THEN
        RETURN false;
    END IF;

    -- Check vehicle status
    IF v_vehicle.status = 'sold' THEN
        RETURN false;
    END IF;

    -- Check for overlapping bookings
    SELECT COUNT(*) INTO v_overlap_count
    FROM rental_bookings
    WHERE vehicle_id = p_vehicle_id
      AND status IN ('paid', 'akan_datang', 'berjalan')
      AND (
          (pickup_date, return_date) OVERLAPS (p_pickup_date, p_return_date)
      );

    IF v_overlap_count > 0 THEN
        RETURN false;
    END IF;

    RETURN true;
END;
$$ LANGUAGE plpgsql;


-- ============================================
-- FUNCTION: Get seller total earnings
-- ============================================

CREATE OR REPLACE FUNCTION get_seller_total_earnings(
    p_seller_id INT,
    p_transaction_type VARCHAR(20) DEFAULT NULL
)
RETURNS TABLE(
    total_earnings DECIMAL(12, 2),
    total_commission DECIMAL(12, 2),
    net_earnings DECIMAL(12, 2),
    transaction_count INT
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        COALESCE(SUM(amount), 0) AS total_earnings,
        COALESCE(SUM(commission_amount), 0) AS total_commission,
        COALESCE(SUM(net_amount), 0) AS net_earnings,
        COUNT(*)::INT AS transaction_count
    FROM transaction_logs
    WHERE user_id = p_seller_id
      AND transaction_type LIKE COALESCE(p_transaction_type || '%', '%payment%')
      AND status = 'completed';
END;
$$ LANGUAGE plpgsql;


-- ============================================
-- FUNCTION: Get seller rating and review stats
-- ============================================

CREATE OR REPLACE FUNCTION get_seller_rating_stats(
    p_seller_id INT
)
RETURNS TABLE(
    avg_rating DECIMAL(3, 2),
    total_reviews INT,
    rating_5_count INT,
    rating_4_count INT,
    rating_3_count INT,
    rating_2_count INT,
    rating_1_count INT
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        COALESCE(AVG(r.overall_rating), 0)::DECIMAL(3, 2) AS avg_rating,
        COUNT(r.id)::INT AS total_reviews,
        COUNT(CASE WHEN r.overall_rating = 5 THEN 1 END)::INT AS rating_5_count,
        COUNT(CASE WHEN r.overall_rating = 4 THEN 1 END)::INT AS rating_4_count,
        COUNT(CASE WHEN r.overall_rating = 3 THEN 1 END)::INT AS rating_3_count,
        COUNT(CASE WHEN r.overall_rating = 2 THEN 1 END)::INT AS rating_2_count,
        COUNT(CASE WHEN r.overall_rating = 1 THEN 1 END)::INT AS rating_1_count
    FROM reviews r
    WHERE r.seller_id = p_seller_id
      AND r.is_visible = true;
END;
$$ LANGUAGE plpgsql;


-- ============================================
-- FUNCTION: Get active sale orders count for vehicle
-- ============================================

CREATE OR REPLACE FUNCTION get_active_sale_orders_count(
    p_vehicle_id INT
)
RETURNS INT AS $$
DECLARE
    v_count INT;
BEGIN
    SELECT COUNT(*) INTO v_count
    FROM sale_orders
    WHERE vehicle_id = p_vehicle_id
      AND status IN ('pending_confirmation', 'pending_payment', 'paid', 'document_processing');

    RETURN COALESCE(v_count, 0);
END;
$$ LANGUAGE plpgsql;


-- ============================================
-- FUNCTION: Get pending documents count for seller
-- ============================================

CREATE OR REPLACE FUNCTION get_pending_documents_count(
    p_seller_id INT
)
RETURNS INT AS $$
DECLARE
    v_count INT;
BEGIN
    SELECT COUNT(*) INTO v_count
    FROM sale_orders
    WHERE seller_id = p_seller_id
      AND status = 'document_processing'
      AND (
          bpkb_transferred = false OR
          stnk_transferred = false OR
          faktur_transferred = false OR
          pajak_transferred = false
      );

    RETURN COALESCE(v_count, 0);
END;
$$ LANGUAGE plpgsql;


-- ============================================
-- FUNCTION: Search vehicles with filters (for API)
-- ============================================

CREATE OR REPLACE FUNCTION search_vehicles(
    p_category VARCHAR(10) DEFAULT NULL,
    p_city VARCHAR(100) DEFAULT NULL,
    p_brand VARCHAR(100) DEFAULT NULL,
    p_model VARCHAR(100) DEFAULT NULL,
    p_min_price DECIMAL(12, 2) DEFAULT NULL,
    p_max_price DECIMAL(12, 2) DEFAULT NULL,
    p_min_year INT DEFAULT NULL,
    p_max_year INT DEFAULT NULL,
    p_transmission VARCHAR(20) DEFAULT NULL,
    p_is_luxury BOOLEAN DEFAULT NULL,
    p_status VARCHAR(20) DEFAULT 'available',
    p_limit INT DEFAULT 20,
    p_offset INT DEFAULT 0
)
RETURNS TABLE(
    id INT,
    title VARCHAR(255),
    category VARCHAR(10),
    price DECIMAL(12, 2),
    brand VARCHAR(100),
    model VARCHAR(100),
    year INT,
    city VARCHAR(100),
    rating DECIMAL(3, 2),
    review_count INT,
    photos JSONB,
    status VARCHAR(20)
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        v.id,
        v.title,
        v.category,
        v.price,
        v.brand,
        v.model,
        v.year,
        v.city,
        v.rating,
        v.review_count,
        v.photos,
        v.status
    FROM vehicles v
    WHERE
        (p_category IS NULL OR v.category = p_category)
        AND (p_city IS NULL OR v.city ILIKE '%' || p_city || '%')
        AND (p_brand IS NULL OR v.brand ILIKE '%' || p_brand || '%')
        AND (p_model IS NULL OR v.model ILIKE '%' || p_model || '%')
        AND (p_min_price IS NULL OR v.price >= p_min_price)
        AND (p_max_price IS NULL OR v.price <= p_max_price)
        AND (p_min_year IS NULL OR v.year >= p_min_year)
        AND (p_max_year IS NULL OR v.year <= p_max_year)
        AND (p_transmission IS NULL OR v.transmission = p_transmission)
        AND (p_is_luxury IS NULL OR v.is_luxury = p_is_luxury)
        AND (p_status IS NULL OR v.status = p_status)
    ORDER BY v.created_at DESC
    LIMIT p_limit
    OFFSET p_offset;
END;
$$ LANGUAGE plpgsql;
