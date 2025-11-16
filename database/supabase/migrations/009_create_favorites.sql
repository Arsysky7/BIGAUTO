-- /PPL/database/supabase/migrations/009_create_favorites.sql 
-- Deskripsi: Wishlist mobil customer

CREATE TABLE favorites (
    id SERIAL PRIMARY KEY,
    customer_id INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    vehicle_id INT NOT NULL REFERENCES vehicles(id) ON DELETE CASCADE,
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    
    
    UNIQUE(customer_id, vehicle_id)
);

-- Index untuk performa
CREATE INDEX idx_favorites_customer_id ON favorites(customer_id);
CREATE INDEX idx_favorites_vehicle_id ON favorites(vehicle_id);