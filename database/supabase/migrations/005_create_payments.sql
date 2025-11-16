-- PPL/database/supabase/migrations/005_create_payments.sql

-- Table: payments
-- Deskripsi: Payment transactions (Midtrans)

CREATE TABLE payments (
    id SERIAL PRIMARY KEY,
    
    booking_id INT NOT NULL REFERENCES rental_bookings(id) ON DELETE CASCADE,
    
    order_id VARCHAR(50) UNIQUE NOT NULL,
    
    -- Midtrans info
    transaction_id VARCHAR(255),
    va_number VARCHAR(50),
    bank VARCHAR(50),
    payment_type VARCHAR(50),
    
    -- Amount
    gross_amount DECIMAL(12, 2) NOT NULL,
    
    -- Status
    status VARCHAR(20) NOT NULL DEFAULT 'pending' CHECK (
        status IN ('pending', 'success', 'failed', 'expired', 'refunded')
    ),
    
    -- Refund
    refund_amount DECIMAL(12, 2),
    refund_reason TEXT,
    
    -- Timestamps
    paid_at TIMESTAMPTZ,
    expired_at TIMESTAMPTZ,
    refunded_at TIMESTAMPTZ,
    
    -- Receipt
    receipt_pdf_path VARCHAR(500),
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);