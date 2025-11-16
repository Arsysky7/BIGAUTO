-- PPL/database/supabase/migrations/019_create_vehicle_brands_models.sql
-- Table: vehicle_brands
-- Deskripsi: Master data merk mobil

CREATE TABLE vehicle_brands (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) UNIQUE NOT NULL,
    logo_url VARCHAR(500),
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Table: vehicle_models
-- Deskripsi: Master data model mobil per merk

CREATE TABLE vehicle_models (
    id SERIAL PRIMARY KEY,
    brand_id INT NOT NULL REFERENCES vehicle_brands(id) ON DELETE CASCADE,
    name VARCHAR(100) NOT NULL,
    vehicle_type VARCHAR(50),
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    
    UNIQUE(brand_id, name)
);

-- Index untuk performa
CREATE INDEX idx_vehicle_models_brand_id ON vehicle_models(brand_id);

-- Seed data contoh
INSERT INTO vehicle_brands (name) VALUES 
    ('Toyota'), ('Honda'), ('Mitsubishi'), ('Suzuki'), 
    ('Daihatsu'), ('Nissan'), ('BMW'), ('Mercedes-Benz');

INSERT INTO vehicle_models (brand_id, name, vehicle_type) VALUES
    (1, 'Avanza', 'MPV'), (1, 'Innova', 'MPV'), (1, 'Fortuner', 'SUV'),
    (2, 'Brio', 'Hatchback'), (2, 'Jazz', 'Hatchback'), (2, 'CR-V', 'SUV'),
    (3, 'Pajero Sport', 'SUV'), (3, 'Xpander', 'MPV');