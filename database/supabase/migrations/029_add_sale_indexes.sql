-- PPL/database/supabase/migrations/029_add_sale_indexes.sql
-- Additional indexes untuk optimize sale order queries

-- Vehicles: Optimize filter jual beli
CREATE INDEX idx_vehicles_sale_filters ON vehicles(category, status, city, brand, model)
    WHERE category = 'sale' AND status = 'available';

CREATE INDEX idx_vehicles_sale_price ON vehicles(category, price)
    WHERE category = 'sale';

CREATE INDEX idx_vehicles_sale_year ON vehicles(category, year DESC)
    WHERE category = 'sale';

CREATE INDEX idx_vehicles_sale_mileage ON vehicles(category, mileage)
    WHERE category = 'sale' AND mileage IS NOT NULL;

-- Sale orders: Optimize dashboard queries
CREATE INDEX idx_sale_orders_seller_pending ON sale_orders(seller_id, created_at DESC)
    WHERE status IN ('pending_confirmation', 'pending_payment');

CREATE INDEX idx_sale_orders_buyer_active ON sale_orders(buyer_id, created_at DESC)
    WHERE status NOT IN ('completed', 'cancelled', 'rejected');

CREATE INDEX idx_sale_orders_processing ON sale_orders(seller_id, document_transfer_started_at)
    WHERE status = 'document_processing';

-- Payments: Optimize sale payment queries
CREATE INDEX idx_payments_sale_pending ON payments(sale_order_id, created_at DESC)
    WHERE payment_for_type = 'sale' AND status = 'pending';

CREATE INDEX idx_payments_sale_success ON payments(sale_order_id, paid_at DESC)
    WHERE payment_for_type = 'sale' AND status = 'success';

-- Reviews: Optimize sale review queries
CREATE INDEX idx_reviews_sale_visible ON reviews(vehicle_id, created_at DESC)
    WHERE review_for_type = 'sale' AND is_visible = true;

-- Transaction logs: Optimize financial reporting
CREATE INDEX idx_transaction_sale_seller ON transaction_logs(user_id, created_at DESC)
    WHERE transaction_type = 'sale_payment';

CREATE INDEX idx_transaction_sale_amount ON transaction_logs(transaction_type, amount, created_at DESC)
    WHERE transaction_type IN ('sale_payment', 'commission_deduction');
