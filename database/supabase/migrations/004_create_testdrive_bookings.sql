-- PPL/database/supabase/migrations/004_create_testdrive_bookings.sql

-- Table: testdrive_bookings
-- Deskripsi: Booking test drive untuk jual beli

CREATE TABLE testdrive_bookings (
    id SERIAL PRIMARY KEY,
    vehicle_id INT NOT NULL REFERENCES vehicles(id) ON DELETE CASCADE,
    customer_id INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    seller_id INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    
    -- Schedule
    requested_date TIMESTAMPTZ NOT NULL,
    requested_time VARCHAR(10) NOT NULL, 
    
    -- Reschedule slots (jika seller reschedule)
    reschedule_slots JSONB, 
    
    -- Customer info
    customer_name VARCHAR(255) NOT NULL,
    customer_phone VARCHAR(20) NOT NULL,
    customer_email VARCHAR(255) NOT NULL,
    notes TEXT,
    
    -- Status
    status VARCHAR(40) NOT NULL DEFAULT 'menunggu_konfirmasi' CHECK (
        status IN (
            'menunggu_konfirmasi',
            'seller_reschedule',
            'diterima',
            'selesai',
            'cancelled',
            'timeout'
        )
    ),
    
    -- Timeout tracking (max 2 jam)
    timeout_at TIMESTAMPTZ,
    
    -- Cancel info
    cancel_reason TEXT,
    cancelled_at TIMESTAMPTZ,
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);