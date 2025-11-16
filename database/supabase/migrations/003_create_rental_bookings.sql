-- PPL/database/supabase/migrations/003_create_rental_bookings.sql

-- Table: rental_bookings
-- Deskripsi: Booking rental mobil

CREATE TABLE rental_bookings (
    id SERIAL PRIMARY KEY,
    vehicle_id INT NOT NULL REFERENCES vehicles(id) ON DELETE CASCADE,
    customer_id INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    seller_id INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    
    -- Order info
    order_id VARCHAR(50) UNIQUE NOT NULL,
    
    -- Booking dates
    pickup_date TIMESTAMPTZ NOT NULL,
    return_date TIMESTAMPTZ NOT NULL,
    
    -- Actual dates (diisi saat validasi)
    actual_pickup_at TIMESTAMPTZ,
    actual_return_at TIMESTAMPTZ,
    
    -- Customer info
    customer_name VARCHAR(255) NOT NULL,
    customer_phone VARCHAR(20) NOT NULL,
    customer_email VARCHAR(255) NOT NULL,
    ktp_photo VARCHAR(500),
    
    -- Booking details
    total_days INT NOT NULL,
    price_per_day DECIMAL(12, 2) NOT NULL,
    total_price DECIMAL(12, 2) NOT NULL,
    notes TEXT,
    
    -- Status
    status VARCHAR(30) NOT NULL DEFAULT 'pending_payment' CHECK (
        status IN (
            'pending_payment',
            'paid',
            'akan_datang',
            'berjalan',
            'selesai',
            'cancelled'
        )
    ),
    
    -- Cancel info
    cancel_reason TEXT,
    cancelled_at TIMESTAMPTZ,
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);