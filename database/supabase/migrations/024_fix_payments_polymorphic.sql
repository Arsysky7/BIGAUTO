-- PPL/database/supabase/migrations/024_fix_payments_polymorphic.sql
-- Fix: payments table untuk support polymorphic (rental & sale)

-- Step 1: Rename booking_id menjadi rental_booking_id
ALTER TABLE payments
    RENAME COLUMN booking_id TO rental_booking_id;

-- Step 2: Make rental_booking_id nullable (karena bisa jadi sale_order_id)
ALTER TABLE payments
    ALTER COLUMN rental_booking_id DROP NOT NULL;

-- Step 3: Add sale_order_id column
ALTER TABLE payments
    ADD COLUMN sale_order_id INT REFERENCES sale_orders(id) ON DELETE CASCADE;

-- Step 4: Add payment_for_type untuk lebih jelas
ALTER TABLE payments
    ADD COLUMN payment_for_type VARCHAR(20) CHECK (payment_for_type IN ('rental', 'sale'));

-- Step 5: Add constraint - harus salah satu, tidak boleh kosong atau keduanya
ALTER TABLE payments
    ADD CONSTRAINT payment_reference_check CHECK (
        (rental_booking_id IS NOT NULL AND sale_order_id IS NULL AND payment_for_type = 'rental') OR
        (rental_booking_id IS NULL AND sale_order_id IS NOT NULL AND payment_for_type = 'sale')
    );

-- Step 6: Update existing data - set payment_for_type untuk data lama
UPDATE payments
SET payment_for_type = 'rental'
WHERE rental_booking_id IS NOT NULL;

-- Step 7: Add indexes untuk sale_order_id
CREATE INDEX idx_payments_sale_order_id ON payments(sale_order_id);
CREATE INDEX idx_payments_type ON payments(payment_for_type);

-- Step 8: Add composite index untuk query umum
CREATE INDEX idx_payments_type_status ON payments(payment_for_type, status);

-- Comments
COMMENT ON COLUMN payments.rental_booking_id IS 'Reference ke rental_bookings (jika payment untuk rental)';
COMMENT ON COLUMN payments.sale_order_id IS 'Reference ke sale_orders (jika payment untuk jual beli)';
COMMENT ON COLUMN payments.payment_for_type IS 'Tipe payment: rental atau sale';
