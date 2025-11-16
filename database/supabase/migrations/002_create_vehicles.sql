-- PPL/database/supabase/migrations/002_create_vehicles.sql

-- Table: vehicles
-- Deskripsi: Produk mobil (rental & jual beli)

CREATE TABLE vehicles (
    id SERIAL PRIMARY KEY,
    seller_id INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    
    -- Basic info
    title VARCHAR(255) NOT NULL,
    category VARCHAR(10) NOT NULL CHECK (category IN ('rental', 'sale')),
    price DECIMAL(12, 2) NOT NULL,
    
    -- Spesifikasi
    brand VARCHAR(100) NOT NULL,
    model VARCHAR(100) NOT NULL,
    year INT NOT NULL,
    transmission VARCHAR(20) CHECK (transmission IN ('manual', 'automatic')),
    fuel_type VARCHAR(20) CHECK (fuel_type IN ('bensin', 'diesel', 'hybrid', 'electric')),
    engine_capacity INT, -- cc
    mileage INT, -- km (untuk jual beli)
    
    -- Kapasitas
    seats INT NOT NULL,
    doors INT,
    luggage_capacity INT,
    
    -- Kategori
    vehicle_type VARCHAR(50) NOT NULL, -- MPV, SUV, Sedan, dll
    is_luxury BOOLEAN DEFAULT false,
    
    -- Kondisi (jual beli)
    is_flood_free BOOLEAN DEFAULT false,
    tax_active BOOLEAN DEFAULT false,
    has_bpkb BOOLEAN DEFAULT false,
    has_stnk BOOLEAN DEFAULT false,
    
    -- Deskripsi
    description TEXT,
    rental_terms TEXT,
    
    -- Lokasi
    city VARCHAR(100) NOT NULL,
    address TEXT NOT NULL,
    latitude DECIMAL(10, 8),
    longitude DECIMAL(11, 8),
    area_coverage JSONB, -- Array zona (rental)
    
    -- Media
    photos JSONB NOT NULL, -- Array URL foto
    
    -- Status
    status VARCHAR(20) DEFAULT 'available' CHECK (status IN ('available', 'sold')),
    
    -- Rating
    rating DECIMAL(3, 2) DEFAULT 0.0,
    review_count INT DEFAULT 0,
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);