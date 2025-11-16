-- PPL/database/supabase/migrations/006_create_reviews.sql

-- Table: reviews
-- Deskripsi: Rating & review setelah rental selesai

CREATE TABLE reviews (
    id SERIAL PRIMARY KEY,
    booking_id INT NOT NULL REFERENCES rental_bookings(id) ON DELETE CASCADE,
    vehicle_id INT NOT NULL REFERENCES vehicles(id) ON DELETE CASCADE,
    seller_id INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    customer_id INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    
    -- Rating
    overall_rating INT NOT NULL CHECK (overall_rating BETWEEN 1 AND 5),
    
    -- Detail rating (optional)
    vehicle_condition_rating INT CHECK (vehicle_condition_rating BETWEEN 1 AND 5),
    cleanliness_rating INT CHECK (cleanliness_rating BETWEEN 1 AND 5),
    service_rating INT CHECK (service_rating BETWEEN 1 AND 5),
    
    -- Review
    comment TEXT,
    photos JSONB,
    
    -- Moderation
    is_visible BOOLEAN DEFAULT true,
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    
    -- 1 booking = 1 review
    UNIQUE(booking_id)
);