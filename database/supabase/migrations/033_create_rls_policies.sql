-- RLS Policies untuk semua tables sesuai business rules
-- NOTE: Backend services pakai service role key (bypass RLS)
-- RLS ini untuk safety net jika ada direct client access

-- ============================================
-- USERS TABLE
-- ============================================
-- User bisa read own profile
CREATE POLICY "Users can read own profile" ON users
  FOR SELECT USING (id = current_user_id());

-- User bisa update own profile
CREATE POLICY "Users can update own profile" ON users
  FOR UPDATE USING (id = current_user_id());

-- Service role bisa insert (register)
-- Note: Service role bypass RLS, jadi tidak perlu policy INSERT

-- ============================================
-- EMAIL VERIFICATIONS
-- ============================================
CREATE POLICY "Service manages email verifications" ON email_verifications
  FOR ALL USING (true);

-- ============================================
-- LOGIN OTPS
-- ============================================
CREATE POLICY "Service manages OTPs" ON login_otps
  FOR ALL USING (true);

-- ============================================
-- USER SESSIONS
-- ============================================
-- User bisa read own sessions
CREATE POLICY "Users can read own sessions" ON user_sessions
  FOR SELECT USING (user_id = current_user_id());

-- User bisa delete own sessions (logout)
CREATE POLICY "Users can delete own sessions" ON user_sessions
  FOR DELETE USING (user_id = current_user_id());

-- ============================================
-- VEHICLES
-- ============================================
-- Everyone bisa read vehicles yang available
CREATE POLICY "Anyone can view available vehicles" ON vehicles
  FOR SELECT USING (status IN ('available', 'pending_sale', 'sold'));

-- Seller bisa create vehicle
CREATE POLICY "Sellers can create vehicles" ON vehicles
  FOR INSERT WITH CHECK (current_user_is_seller() AND seller_id = current_user_id());

-- Seller bisa update own vehicles
CREATE POLICY "Sellers can update own vehicles" ON vehicles
  FOR UPDATE USING (seller_id = current_user_id());

-- Seller bisa delete own vehicles
CREATE POLICY "Sellers can delete own vehicles" ON vehicles
  FOR DELETE USING (seller_id = current_user_id());

-- ============================================
-- VEHICLE BRANDS & MODELS (Master Data)
-- ============================================
CREATE POLICY "Anyone can read brands" ON vehicle_brands
  FOR SELECT USING (true);

CREATE POLICY "Anyone can read models" ON vehicle_models
  FOR SELECT USING (true);

-- ============================================
-- RENTAL BOOKINGS
-- ============================================
-- Customer bisa read own bookings
CREATE POLICY "Customers can read own bookings" ON rental_bookings
  FOR SELECT USING (customer_id = current_user_id());

-- Seller bisa read bookings untuk their vehicles
CREATE POLICY "Sellers can read bookings for their vehicles" ON rental_bookings
  FOR SELECT USING (
    EXISTS (SELECT 1 FROM vehicles WHERE vehicles.id = rental_bookings.vehicle_id AND vehicles.seller_id = current_user_id())
  );

-- Customer bisa create booking
CREATE POLICY "Customers can create bookings" ON rental_bookings
  FOR INSERT WITH CHECK (customer_id = current_user_id());

-- Customer bisa update own booking (cancel)
CREATE POLICY "Customers can update own bookings" ON rental_bookings
  FOR UPDATE USING (customer_id = current_user_id());

-- Seller bisa update bookings untuk their vehicles (validate pickup/return)
CREATE POLICY "Sellers can update bookings for their vehicles" ON rental_bookings
  FOR UPDATE USING (
    EXISTS (SELECT 1 FROM vehicles WHERE vehicles.id = rental_bookings.vehicle_id AND vehicles.seller_id = current_user_id())
  );

-- ============================================
-- TEST DRIVE BOOKINGS
-- ============================================
CREATE POLICY "Customers can read own testdrive bookings" ON testdrive_bookings
  FOR SELECT USING (customer_id = current_user_id());

CREATE POLICY "Sellers can read testdrive bookings for their vehicles" ON testdrive_bookings
  FOR SELECT USING (
    EXISTS (SELECT 1 FROM vehicles WHERE vehicles.id = testdrive_bookings.vehicle_id AND vehicles.seller_id = current_user_id())
  );

CREATE POLICY "Customers can create testdrive bookings" ON testdrive_bookings
  FOR INSERT WITH CHECK (customer_id = current_user_id());

CREATE POLICY "Customers can update own testdrive bookings" ON testdrive_bookings
  FOR UPDATE USING (customer_id = current_user_id());

CREATE POLICY "Sellers can update testdrive bookings for their vehicles" ON testdrive_bookings
  FOR UPDATE USING (
    EXISTS (SELECT 1 FROM vehicles WHERE vehicles.id = testdrive_bookings.vehicle_id AND vehicles.seller_id = current_user_id())
  );

-- ============================================
-- SALE ORDERS
-- ============================================
CREATE POLICY "Buyers can read own sale orders" ON sale_orders
  FOR SELECT USING (buyer_id = current_user_id());

CREATE POLICY "Sellers can read sale orders for their vehicles" ON sale_orders
  FOR SELECT USING (seller_id = current_user_id());

CREATE POLICY "Buyers can create sale orders" ON sale_orders
  FOR INSERT WITH CHECK (buyer_id = current_user_id());

CREATE POLICY "Buyers can update own sale orders" ON sale_orders
  FOR UPDATE USING (buyer_id = current_user_id());

CREATE POLICY "Sellers can update sale orders for their vehicles" ON sale_orders
  FOR UPDATE USING (seller_id = current_user_id());

-- ============================================
-- PAYMENTS
-- ============================================
-- Customers can read payments for their rentals
CREATE POLICY "Customers can read rental payments" ON payments
  FOR SELECT USING (
    payment_for_type = 'rental' AND EXISTS (
      SELECT 1 FROM rental_bookings rb
      WHERE rb.id = payments.rental_booking_id AND rb.customer_id = current_user_id()
    )
  );

-- Customers can read payments for their sale orders
CREATE POLICY "Customers can read sale payments" ON payments
  FOR SELECT USING (
    payment_for_type = 'sale' AND EXISTS (
      SELECT 1 FROM sale_orders so
      WHERE so.id = payments.sale_order_id AND so.buyer_id = current_user_id()
    )
  );

-- Sellers can read payments for their vehicle rentals
CREATE POLICY "Sellers can read rental payments" ON payments
  FOR SELECT USING (
    payment_for_type = 'rental' AND EXISTS (
      SELECT 1 FROM rental_bookings rb
      JOIN vehicles v ON v.id = rb.vehicle_id
      WHERE rb.id = payments.rental_booking_id AND v.seller_id = current_user_id()
    )
  );

-- Sellers can read payments for their sales
CREATE POLICY "Sellers can read sale payments" ON payments
  FOR SELECT USING (
    payment_for_type = 'sale' AND EXISTS (
      SELECT 1 FROM sale_orders so
      WHERE so.id = payments.sale_order_id AND so.seller_id = current_user_id()
    )
  );

-- ============================================
-- SELLER BALANCE
-- ============================================
CREATE POLICY "Sellers can read own balance" ON seller_balance
  FOR SELECT USING (seller_id = current_user_id());

-- ============================================
-- WITHDRAWALS
-- ============================================
CREATE POLICY "Sellers can read own withdrawals" ON withdrawals
  FOR SELECT USING (seller_id = current_user_id());

CREATE POLICY "Sellers can create withdrawals" ON withdrawals
  FOR INSERT WITH CHECK (seller_id = current_user_id());

-- ============================================
-- TRANSACTION LOGS
-- ============================================
CREATE POLICY "Sellers can read own transaction logs" ON transaction_logs
  FOR SELECT USING (seller_id = current_user_id());

-- ============================================
-- COMMISSION SETTINGS (Read-only untuk users)
-- ============================================
CREATE POLICY "Anyone can read active commission settings" ON commission_settings
  FOR SELECT USING (is_active = true);

-- ============================================
-- REVIEWS
-- ============================================
-- Anyone bisa read reviews
CREATE POLICY "Anyone can read reviews" ON reviews
  FOR SELECT USING (true);

-- Customer bisa create review (setelah transaksi selesai)
CREATE POLICY "Customers can create reviews" ON reviews
  FOR INSERT WITH CHECK (customer_id = current_user_id());

-- Customer bisa update own review
CREATE POLICY "Customers can update own reviews" ON reviews
  FOR UPDATE USING (customer_id = current_user_id());

-- ============================================
-- FAVORITES
-- ============================================
CREATE POLICY "Customers can read own favorites" ON favorites
  FOR SELECT USING (customer_id = current_user_id());

CREATE POLICY "Customers can add favorites" ON favorites
  FOR INSERT WITH CHECK (customer_id = current_user_id());

CREATE POLICY "Customers can remove favorites" ON favorites
  FOR DELETE USING (customer_id = current_user_id());

-- ============================================
-- CONVERSATIONS
-- ============================================
CREATE POLICY "Users can read own conversations" ON conversations
  FOR SELECT USING (customer_id = current_user_id() OR seller_id = current_user_id());

CREATE POLICY "Users can create conversations" ON conversations
  FOR INSERT WITH CHECK (customer_id = current_user_id() OR seller_id = current_user_id());

-- ============================================
-- MESSAGES
-- ============================================
CREATE POLICY "Users can read messages in their conversations" ON messages
  FOR SELECT USING (
    EXISTS (
      SELECT 1 FROM conversations
      WHERE conversations.id = messages.conversation_id
      AND (conversations.customer_id = current_user_id() OR conversations.seller_id = current_user_id())
    )
  );

CREATE POLICY "Users can send messages in their conversations" ON messages
  FOR INSERT WITH CHECK (
    sender_id = current_user_id() AND
    EXISTS (
      SELECT 1 FROM conversations
      WHERE conversations.id = conversation_id
      AND (conversations.customer_id = current_user_id() OR conversations.seller_id = current_user_id())
    )
  );

-- ============================================
-- NOTIFICATIONS
-- ============================================
CREATE POLICY "Users can read own notifications" ON notifications
  FOR SELECT USING (user_id = current_user_id());

CREATE POLICY "Users can update own notifications" ON notifications
  FOR UPDATE USING (user_id = current_user_id());

-- ============================================
-- CITIES (Master Data)
-- ============================================
CREATE POLICY "Anyone can read cities" ON cities
  FOR SELECT USING (true);

-- ============================================
-- AUDIT LOGS
-- ============================================
CREATE POLICY "Service manages audit logs" ON audit_logs
  FOR ALL USING (true);

-- ============================================
-- RATE LIMITS
-- ============================================
CREATE POLICY "Service manages rate limits" ON rate_limits
  FOR ALL USING (true);
