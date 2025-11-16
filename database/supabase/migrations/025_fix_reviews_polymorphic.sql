-- PPL/database/supabase/migrations/025_fix_reviews_polymorphic.sql
-- Fix: reviews table untuk support polymorphic (rental & sale)

-- Step 1: Rename booking_id menjadi rental_booking_id
ALTER TABLE reviews
    RENAME COLUMN booking_id TO rental_booking_id;

-- Step 2: Make rental_booking_id nullable
ALTER TABLE reviews
    ALTER COLUMN rental_booking_id DROP NOT NULL;

-- Step 3: Drop unique constraint on old booking_id
ALTER TABLE reviews
    DROP CONSTRAINT reviews_booking_id_key;

-- Step 4: Add sale_order_id column
ALTER TABLE reviews
    ADD COLUMN sale_order_id INT REFERENCES sale_orders(id) ON DELETE CASCADE;

-- Step 5: Add review_for_type untuk lebih jelas
ALTER TABLE reviews
    ADD COLUMN review_for_type VARCHAR(20) CHECK (review_for_type IN ('rental', 'sale'));

-- Step 6: Add constraint - harus salah satu, tidak boleh kosong atau keduanya
ALTER TABLE reviews
    ADD CONSTRAINT review_reference_check CHECK (
        (rental_booking_id IS NOT NULL AND sale_order_id IS NULL AND review_for_type = 'rental') OR
        (rental_booking_id IS NULL AND sale_order_id IS NOT NULL AND review_for_type = 'sale')
    );

-- Step 7: Add unique constraint - 1 rental booking atau 1 sale order = 1 review
CREATE UNIQUE INDEX idx_reviews_rental_unique ON reviews(rental_booking_id) WHERE rental_booking_id IS NOT NULL;
CREATE UNIQUE INDEX idx_reviews_sale_unique ON reviews(sale_order_id) WHERE sale_order_id IS NOT NULL;

-- Step 8: Update existing data - set review_for_type untuk data lama
UPDATE reviews
SET review_for_type = 'rental'
WHERE rental_booking_id IS NOT NULL;

-- Step 9: Add indexes untuk sale_order_id
CREATE INDEX idx_reviews_sale_order_id ON reviews(sale_order_id);
CREATE INDEX idx_reviews_type ON reviews(review_for_type);

-- Step 10: Rename detail rating untuk lebih generic
-- vehicle_condition_rating sudah cocok untuk rental & sale
-- cleanliness_rating diganti jadi accuracy_rating (untuk sale = sesuai deskripsi)
ALTER TABLE reviews
    RENAME COLUMN cleanliness_rating TO accuracy_rating;

-- Step 11: Add comments untuk clarity
COMMENT ON COLUMN reviews.rental_booking_id IS 'Reference ke rental_bookings (jika review untuk rental)';
COMMENT ON COLUMN reviews.sale_order_id IS 'Reference ke sale_orders (jika review untuk jual beli)';
COMMENT ON COLUMN reviews.review_for_type IS 'Tipe review: rental atau sale';
COMMENT ON COLUMN reviews.vehicle_condition_rating IS 'Rating kondisi mobil (1-5)';
COMMENT ON COLUMN reviews.accuracy_rating IS 'Rating kebersihan (rental) atau sesuai deskripsi (sale) (1-5)';
COMMENT ON COLUMN reviews.service_rating IS 'Rating pelayanan seller (1-5)';
