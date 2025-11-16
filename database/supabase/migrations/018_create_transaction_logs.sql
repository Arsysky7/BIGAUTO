-- PPL/database/supabase/migrations/018_create_transaction_logs.sql
-- Table: transaction_logs  
-- Deskripsi: Log semua transaksi finansial

CREATE TABLE transaction_logs (
    id SERIAL PRIMARY KEY,
    
    -- Transaction info
    transaction_type VARCHAR(50) NOT NULL CHECK (
        transaction_type IN (
            'rental_payment',
            'rental_refund', 
            'seller_credit',
            'seller_withdrawal',
            'commission_deduction'
        )
    ),
    
    -- Related entities
    user_id INT REFERENCES users(id) ON DELETE SET NULL,
    booking_id INT REFERENCES rental_bookings(id) ON DELETE SET NULL,
    payment_id INT REFERENCES payments(id) ON DELETE SET NULL,
    withdrawal_id INT REFERENCES withdrawals(id) ON DELETE SET NULL,
    
    -- Amount
    amount DECIMAL(12, 2) NOT NULL,
    commission_amount DECIMAL(12, 2),
    net_amount DECIMAL(12, 2),
    
    -- Status
    status VARCHAR(20) DEFAULT 'pending' CHECK (
        status IN ('pending', 'completed', 'failed', 'reversed')
    ),
    
    -- Metadata
    metadata JSONB,
    notes TEXT,
    
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Index untuk performa
CREATE INDEX idx_transaction_logs_user_id ON transaction_logs(user_id);
CREATE INDEX idx_transaction_logs_type ON transaction_logs(transaction_type);
CREATE INDEX idx_transaction_logs_status ON transaction_logs(status);
CREATE INDEX idx_transaction_logs_created_at ON transaction_logs(created_at DESC);