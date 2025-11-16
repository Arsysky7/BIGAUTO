-- PPL/database/supabase/migrations/026_update_transaction_logs.sql
-- Update: transaction_logs untuk support sale payment types

-- Step 1: Add sale_order_id reference
ALTER TABLE transaction_logs
    ADD COLUMN sale_order_id INT REFERENCES sale_orders(id) ON DELETE SET NULL;

-- Step 2: Drop old constraint
ALTER TABLE transaction_logs
    DROP CONSTRAINT transaction_logs_transaction_type_check;

-- Step 3: Add new constraint with sale types
ALTER TABLE transaction_logs
    ADD CONSTRAINT transaction_logs_transaction_type_check CHECK (
        transaction_type IN (
            'rental_payment',       -- Payment dari customer untuk rental
            'rental_refund',        -- Refund rental ke customer
            'sale_payment',         -- Payment dari customer untuk jual beli (NEW!)
            'seller_credit',        -- Credit ke seller (dari rental atau sale)
            'seller_withdrawal',    -- Withdrawal seller ke bank
            'commission_deduction'  -- Potongan komisi platform
        )
    );

-- Step 4: Add index untuk sale_order_id
CREATE INDEX idx_transaction_logs_sale_order_id ON transaction_logs(sale_order_id);

-- Step 5: Add composite index untuk query reporting
CREATE INDEX idx_transaction_logs_type_user ON transaction_logs(transaction_type, user_id);
CREATE INDEX idx_transaction_logs_type_date ON transaction_logs(transaction_type, created_at DESC);

COMMENT ON COLUMN transaction_logs.sale_order_id IS 'Reference ke sale_orders (jika transaksi terkait jual beli)';
