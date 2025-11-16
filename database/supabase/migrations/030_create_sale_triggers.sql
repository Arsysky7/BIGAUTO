-- PPL/database/supabase/migrations/030_create_sale_triggers.sql
-- Triggers dan Functions untuk automation sale order flow

-- ============================================
-- FUNCTION: Update vehicle status when sale order status changes
-- ============================================

CREATE OR REPLACE FUNCTION update_vehicle_status_on_sale()
RETURNS TRIGGER AS $$
BEGIN
    -- When sale order is PAID, set vehicle to PENDING_SALE
    IF NEW.status = 'paid' AND OLD.status != 'paid' THEN
        UPDATE vehicles
        SET status = 'pending_sale', updated_at = NOW()
        WHERE id = NEW.vehicle_id;

    -- When sale order is COMPLETED, set vehicle to SOLD
    ELSIF NEW.status = 'completed' AND OLD.status != 'completed' THEN
        UPDATE vehicles
        SET status = 'sold', updated_at = NOW()
        WHERE id = NEW.vehicle_id;

    -- When sale order is CANCELLED or REJECTED, set vehicle back to AVAILABLE
    ELSIF (NEW.status IN ('cancelled', 'rejected')) AND (OLD.status NOT IN ('cancelled', 'rejected')) THEN
        UPDATE vehicles
        SET status = 'available', updated_at = NOW()
        WHERE id = NEW.vehicle_id;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_vehicle_on_sale_status
AFTER UPDATE OF status ON sale_orders
FOR EACH ROW
EXECUTE FUNCTION update_vehicle_status_on_sale();


-- ============================================
-- FUNCTION: Create transaction log when payment success for sale
-- ============================================

CREATE OR REPLACE FUNCTION create_sale_transaction_on_payment()
RETURNS TRIGGER AS $$
DECLARE
    v_sale_order sale_orders%ROWTYPE;
    v_commission_rate DECIMAL(5, 2);
    v_commission_amount DECIMAL(12, 2);
    v_net_amount DECIMAL(12, 2);
BEGIN
    -- Only process when payment changes to SUCCESS for SALE
    IF NEW.status = 'success' AND NEW.payment_for_type = 'sale' AND OLD.status != 'success' THEN

        -- Get sale order details
        SELECT * INTO v_sale_order FROM sale_orders WHERE id = NEW.sale_order_id;

        -- Get current commission rate for sale
        SELECT commission_percentage INTO v_commission_rate
        FROM commission_settings
        WHERE transaction_type = 'sale'
          AND is_active = true
          AND effective_from <= NOW()
          AND (effective_until IS NULL OR effective_until >= NOW())
        ORDER BY effective_from DESC
        LIMIT 1;

        -- Default to 5% if not found
        IF v_commission_rate IS NULL THEN
            v_commission_rate := 5.00;
        END IF;

        -- Calculate commission and net amount
        v_commission_amount := (v_sale_order.final_price * v_commission_rate / 100);
        v_net_amount := v_sale_order.final_price - v_commission_amount;

        -- Insert transaction log for sale payment
        INSERT INTO transaction_logs (
            transaction_type,
            user_id,
            sale_order_id,
            payment_id,
            amount,
            commission_amount,
            net_amount,
            status,
            metadata,
            notes,
            created_at
        ) VALUES (
            'sale_payment',
            v_sale_order.seller_id,
            v_sale_order.id,
            NEW.id,
            v_sale_order.final_price,
            v_commission_amount,
            v_net_amount,
            'pending', -- Pending sampai dokumen diserahkan
            jsonb_build_object(
                'order_id', v_sale_order.order_id,
                'vehicle_id', v_sale_order.vehicle_id,
                'buyer_id', v_sale_order.buyer_id,
                'commission_rate', v_commission_rate
            ),
            'Payment received for sale order, pending document transfer',
            NOW()
        );

        -- Update seller balance - masuk ke PENDING dulu
        INSERT INTO seller_balance (seller_id, pending_balance, updated_at)
        VALUES (v_sale_order.seller_id, v_net_amount, NOW())
        ON CONFLICT (seller_id) DO UPDATE
        SET pending_balance = seller_balance.pending_balance + v_net_amount,
            updated_at = NOW();

    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_create_sale_transaction
AFTER UPDATE OF status ON payments
FOR EACH ROW
EXECUTE FUNCTION create_sale_transaction_on_payment();


-- ============================================
-- FUNCTION: Transfer balance from pending to available when sale completed
-- ============================================

CREATE OR REPLACE FUNCTION transfer_seller_balance_on_sale_complete()
RETURNS TRIGGER AS $$
DECLARE
    v_net_amount DECIMAL(12, 2);
BEGIN
    -- When sale order status changes to COMPLETED
    IF NEW.status = 'completed' AND OLD.status != 'completed' THEN

        -- Get net amount from transaction_logs
        SELECT net_amount INTO v_net_amount
        FROM transaction_logs
        WHERE sale_order_id = NEW.id
          AND transaction_type = 'sale_payment'
        LIMIT 1;

        IF v_net_amount IS NOT NULL THEN
            -- Transfer from pending to available
            UPDATE seller_balance
            SET pending_balance = pending_balance - v_net_amount,
                available_balance = available_balance + v_net_amount,
                total_earned = total_earned + v_net_amount,
                updated_at = NOW()
            WHERE seller_id = NEW.seller_id;

            -- Update transaction log status to completed
            UPDATE transaction_logs
            SET status = 'completed',
                notes = notes || ' | Document transfer completed, balance available for withdrawal'
            WHERE sale_order_id = NEW.id
              AND transaction_type = 'sale_payment';
        END IF;

    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_transfer_balance_on_sale_complete
AFTER UPDATE OF status ON sale_orders
FOR EACH ROW
EXECUTE FUNCTION transfer_seller_balance_on_sale_complete();


-- ============================================
-- FUNCTION: Auto-update updated_at timestamp
-- ============================================

CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_sale_orders_updated_at
BEFORE UPDATE ON sale_orders
FOR EACH ROW
EXECUTE FUNCTION update_updated_at_column();


-- ============================================
-- FUNCTION: Prevent delete if sale order has payment
-- ============================================

CREATE OR REPLACE FUNCTION prevent_delete_paid_sale_order()
RETURNS TRIGGER AS $$
BEGIN
    IF OLD.status IN ('paid', 'document_processing', 'completed') THEN
        RAISE EXCEPTION 'Cannot delete sale order that has been paid or completed';
    END IF;
    RETURN OLD;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_prevent_delete_paid_sale
BEFORE DELETE ON sale_orders
FOR EACH ROW
EXECUTE FUNCTION prevent_delete_paid_sale_order();


-- ============================================
-- FUNCTION: Auto-generate order_id for sale orders
-- ============================================

CREATE OR REPLACE FUNCTION generate_sale_order_id()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.order_id IS NULL OR NEW.order_id = '' THEN
        NEW.order_id := 'SAL-' || TO_CHAR(NOW(), 'YYYYMMDD') || '-' || LPAD(NEW.id::TEXT, 5, '0');
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_generate_sale_order_id
BEFORE INSERT ON sale_orders
FOR EACH ROW
EXECUTE FUNCTION generate_sale_order_id();


-- ============================================
-- FUNCTION: Update vehicle rating when review added/updated
-- ============================================

CREATE OR REPLACE FUNCTION update_vehicle_rating_on_review()
RETURNS TRIGGER AS $$
DECLARE
    v_avg_rating DECIMAL(3, 2);
    v_review_count INT;
    v_vehicle_id INT;
BEGIN
    -- Get vehicle_id from sale_order or rental_booking
    IF NEW.review_for_type = 'sale' THEN
        SELECT vehicle_id INTO v_vehicle_id FROM sale_orders WHERE id = NEW.sale_order_id;
    ELSE
        SELECT vehicle_id INTO v_vehicle_id FROM rental_bookings WHERE id = NEW.rental_booking_id;
    END IF;

    -- Calculate average rating and count
    SELECT AVG(overall_rating), COUNT(*)
    INTO v_avg_rating, v_review_count
    FROM reviews
    WHERE vehicle_id = v_vehicle_id
      AND is_visible = true;

    -- Update vehicle
    UPDATE vehicles
    SET rating = COALESCE(v_avg_rating, 0),
        review_count = COALESCE(v_review_count, 0),
        updated_at = NOW()
    WHERE id = v_vehicle_id;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_rating_on_review_insert
AFTER INSERT ON reviews
FOR EACH ROW
EXECUTE FUNCTION update_vehicle_rating_on_review();

CREATE TRIGGER trigger_update_rating_on_review_update
AFTER UPDATE OF overall_rating, is_visible ON reviews
FOR EACH ROW
EXECUTE FUNCTION update_vehicle_rating_on_review();


-- ============================================
-- FUNCTION: Update conversation last_message
-- ============================================

CREATE OR REPLACE FUNCTION update_conversation_last_message()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE conversations
    SET last_message = NEW.content,
        last_message_at = NEW.created_at,
        updated_at = NOW()
    WHERE id = NEW.conversation_id;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_conversation_on_message
AFTER INSERT ON messages
FOR EACH ROW
EXECUTE FUNCTION update_conversation_last_message();
