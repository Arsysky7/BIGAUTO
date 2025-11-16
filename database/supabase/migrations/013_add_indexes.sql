-- PPL/database/supabase/migrations/013_add_indexes.sql

-- Indexes untuk performance

-- Users
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_is_seller ON users(is_seller);

-- Vehicles
CREATE INDEX idx_vehicles_seller_id ON vehicles(seller_id);
CREATE INDEX idx_vehicles_category ON vehicles(category);
CREATE INDEX idx_vehicles_city ON vehicles(city);
CREATE INDEX idx_vehicles_brand ON vehicles(brand);
CREATE INDEX idx_vehicles_status ON vehicles(status);
CREATE INDEX idx_vehicles_created_at ON vehicles(created_at DESC);

-- Rental Bookings
CREATE INDEX idx_rental_bookings_vehicle_id ON rental_bookings(vehicle_id);
CREATE INDEX idx_rental_bookings_customer_id ON rental_bookings(customer_id);
CREATE INDEX idx_rental_bookings_seller_id ON rental_bookings(seller_id);
CREATE INDEX idx_rental_bookings_order_id ON rental_bookings(order_id);
CREATE INDEX idx_rental_bookings_status ON rental_bookings(status);
CREATE INDEX idx_rental_bookings_dates ON rental_bookings(pickup_date, return_date);

-- Test Drive Bookings
CREATE INDEX idx_testdrive_bookings_vehicle_id ON testdrive_bookings(vehicle_id);
CREATE INDEX idx_testdrive_bookings_customer_id ON testdrive_bookings(customer_id);
CREATE INDEX idx_testdrive_bookings_seller_id ON testdrive_bookings(seller_id);
CREATE INDEX idx_testdrive_bookings_status ON testdrive_bookings(status);

-- Payments
CREATE INDEX idx_payments_order_id ON payments(order_id);
CREATE INDEX idx_payments_status ON payments(status);

-- Reviews
CREATE INDEX idx_reviews_vehicle_id ON reviews(vehicle_id);
CREATE INDEX idx_reviews_seller_id ON reviews(seller_id);
CREATE INDEX idx_reviews_customer_id ON reviews(customer_id);

-- Messages
CREATE INDEX idx_messages_conversation_id ON messages(conversation_id);
CREATE INDEX idx_messages_sender_id ON messages(sender_id);
CREATE INDEX idx_messages_created_at ON messages(created_at DESC);

-- Notifications
CREATE INDEX idx_notifications_user_id ON notifications(user_id);
CREATE INDEX idx_notifications_is_read ON notifications(is_read);