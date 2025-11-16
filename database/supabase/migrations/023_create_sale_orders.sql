-- PPL/database/supabase/migrations/023_create_sale_orders.sql
-- Table: sale_orders
-- Deskripsi: Order pembelian mobil (jual beli)

CREATE TABLE sale_orders (
    id SERIAL PRIMARY KEY,
    vehicle_id INT NOT NULL REFERENCES vehicles(id) ON DELETE CASCADE,
    buyer_id INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    seller_id INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- Link to test drive (optional)
    testdrive_booking_id INT REFERENCES testdrive_bookings(id) ON DELETE SET NULL,

    -- Order info
    order_id VARCHAR(50) UNIQUE NOT NULL,

    -- Pricing
    asking_price DECIMAL(12, 2) NOT NULL, -- Harga asli dari listing
    offer_price DECIMAL(12, 2), -- Harga nego dari buyer (optional)
    counter_offer_price DECIMAL(12, 2), -- Counter dari seller (optional)
    final_price DECIMAL(12, 2) NOT NULL, -- Harga final yang disepakati

    -- Buyer info
    buyer_name VARCHAR(255) NOT NULL,
    buyer_phone VARCHAR(20) NOT NULL,
    buyer_email VARCHAR(255) NOT NULL,
    buyer_address TEXT,
    buyer_ktp_photo VARCHAR(500), -- Upload KTP untuk verifikasi

    -- Status
    status VARCHAR(30) NOT NULL DEFAULT 'pending_confirmation' CHECK (
        status IN (
            'pending_confirmation', 
            'pending_payment',      
            'paid',                 -- Sudah bayar
            'document_processing',  -- Proses serah terima dokumen
            'completed',            -- Selesai, dokumen sudah diserahkan
            'cancelled',            -- Dibatalkan
            'rejected'              -- Ditolak seller
        )
    ),

    -- Documents tracking
    bpkb_transferred BOOLEAN DEFAULT false,
    stnk_transferred BOOLEAN DEFAULT false,
    faktur_transferred BOOLEAN DEFAULT false,
    pajak_transferred BOOLEAN DEFAULT false,

    -- Timestamps
    created_at TIMESTAMPTZ DEFAULT NOW(),
    confirmed_at TIMESTAMPTZ, -- Seller confirm
    paid_at TIMESTAMPTZ,
    document_transfer_started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    cancelled_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    -- Cancel/Reject info
    cancel_reason TEXT,
    reject_reason TEXT,
    rejected_at TIMESTAMPTZ,

    -- Notes
    buyer_notes TEXT, -- Catatan dari buyer
    seller_notes TEXT -- Catatan dari seller
);

-- Indexes untuk performa
CREATE INDEX idx_sale_orders_vehicle_id ON sale_orders(vehicle_id);
CREATE INDEX idx_sale_orders_buyer_id ON sale_orders(buyer_id);
CREATE INDEX idx_sale_orders_seller_id ON sale_orders(seller_id);
CREATE INDEX idx_sale_orders_order_id ON sale_orders(order_id);
CREATE INDEX idx_sale_orders_status ON sale_orders(status);
CREATE INDEX idx_sale_orders_created_at ON sale_orders(created_at DESC);
CREATE INDEX idx_sale_orders_testdrive ON sale_orders(testdrive_booking_id) WHERE testdrive_booking_id IS NOT NULL;

-- Composite indexes untuk query umum
CREATE INDEX idx_sale_orders_seller_status ON sale_orders(seller_id, status);
CREATE INDEX idx_sale_orders_buyer_status ON sale_orders(buyer_id, status);

-- Comments
COMMENT ON TABLE sale_orders IS 'Order pembelian mobil untuk kategori jual beli';
COMMENT ON COLUMN sale_orders.asking_price IS 'Harga asli dari listing vehicle';
COMMENT ON COLUMN sale_orders.offer_price IS 'Harga penawaran dari buyer (jika nego)';
COMMENT ON COLUMN sale_orders.counter_offer_price IS 'Counter offer dari seller (jika buyer nego)';
COMMENT ON COLUMN sale_orders.final_price IS 'Harga final yang disepakati dan dibayar';
COMMENT ON COLUMN sale_orders.testdrive_booking_id IS 'Link ke test drive booking jika ada';
