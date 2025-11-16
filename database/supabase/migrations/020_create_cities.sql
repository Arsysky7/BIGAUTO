-- PPL/database/supabase/migrations/020_create_cities.sql
-- Table: cities
-- Deskripsi: Master data kota

CREATE TABLE cities (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) UNIQUE NOT NULL,
    province VARCHAR(100) NOT NULL,
    is_major_city BOOLEAN DEFAULT false,
    latitude DECIMAL(10, 8),
    longitude DECIMAL(11, 8),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Index untuk performa
CREATE INDEX idx_cities_name ON cities(name);
CREATE INDEX idx_cities_is_major ON cities(is_major_city);

-- Seed data kota besar
INSERT INTO cities (name, province, is_major_city, latitude, longitude) VALUES
    ('Jakarta', 'DKI Jakarta', true, -6.200000, 106.816666),
    ('Surabaya', 'Jawa Timur', true, -7.257472, 112.752090),
    ('Bandung', 'Jawa Barat', true, -6.917464, 107.619125),
    ('Medan', 'Sumatera Utara', true, 3.595196, 98.672226),
    ('Bali', 'Bali', true, -8.340539, 115.091951),
    ('Yogyakarta', 'DIY', true, -7.795580, 110.369490),
    ('Semarang', 'Jawa Tengah', true, -6.966667, 110.416664),
    ('Makassar', 'Sulawesi Selatan', true, -5.147665, 119.432731);