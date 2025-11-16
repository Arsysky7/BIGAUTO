-- PPL/database/supabase/migrations/028_update_vehicles_status.sql
-- Update: vehicles table status untuk support pending_sale

-- Step 1: Drop old status constraint
ALTER TABLE vehicles
    DROP CONSTRAINT vehicles_status_check;

-- Step 2: Add new status constraint dengan pending_sale
ALTER TABLE vehicles
    ADD CONSTRAINT vehicles_status_check CHECK (
        status IN (
            'available',    -- Tersedia
            'pending_sale', -- Ada order, pending payment atau dokumen (NEW!)
            'sold'          -- Sudah terjual
        )
    );

-- Step 3: Add composite index untuk query common
CREATE INDEX idx_vehicles_category_status ON vehicles(category, status);
CREATE INDEX idx_vehicles_city_status ON vehicles(city, status);

COMMENT ON COLUMN vehicles.status IS 'Status: available (tersedia), pending_sale (ada order pending), sold (terjual)';
