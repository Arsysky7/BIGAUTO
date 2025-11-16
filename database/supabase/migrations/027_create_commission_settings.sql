-- PPL/database/supabase/migrations/027_create_commission_settings.sql
-- Table: commission_settings
-- Deskripsi: Pengaturan komisi platform untuk rental dan sale

CREATE TABLE commission_settings (
    id SERIAL PRIMARY KEY,

    -- Type
    transaction_type VARCHAR(20) NOT NULL CHECK (transaction_type IN ('rental', 'sale')),

    -- Commission rate
    commission_percentage DECIMAL(5, 2) NOT NULL CHECK (commission_percentage >= 0 AND commission_percentage <= 100),

    -- Optional min/max
    min_commission DECIMAL(12, 2) CHECK (min_commission >= 0),
    max_commission DECIMAL(12, 2) CHECK (max_commission >= min_commission),

    -- Validity period
    effective_from TIMESTAMPTZ NOT NULL,
    effective_until TIMESTAMPTZ,

    -- Status
    is_active BOOLEAN DEFAULT true,

    -- Notes
    description TEXT,

    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Index
CREATE INDEX idx_commission_settings_type ON commission_settings(transaction_type);
CREATE INDEX idx_commission_settings_active ON commission_settings(is_active);
CREATE INDEX idx_commission_settings_effective ON commission_settings(effective_from, effective_until);

-- Constraint: tidak boleh ada overlap periode untuk type yang sama
CREATE UNIQUE INDEX idx_commission_no_overlap ON commission_settings(transaction_type, effective_from)
    WHERE is_active = true AND effective_until IS NULL;

-- Seed data: Default commission 5% untuk rental dan sale
INSERT INTO commission_settings (transaction_type, commission_percentage, effective_from, description)
VALUES
    ('rental', 5.00, NOW(), 'Default commission untuk rental mobil'),
    ('sale', 5.00, NOW(), 'Default commission untuk jual beli mobil');

COMMENT ON TABLE commission_settings IS 'Pengaturan komisi platform untuk rental dan jual beli';
COMMENT ON COLUMN commission_settings.commission_percentage IS 'Persentase komisi (5.00 = 5%)';
COMMENT ON COLUMN commission_settings.min_commission IS 'Minimal komisi dalam rupiah (optional)';
COMMENT ON COLUMN commission_settings.max_commission IS 'Maksimal komisi dalam rupiah (optional)';
