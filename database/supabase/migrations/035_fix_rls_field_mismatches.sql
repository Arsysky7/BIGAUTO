-- Fix RLS Policy Field Mismatches


-- Drop old policies (created dengan field salah)
DROP POLICY IF EXISTS "Customers can create reviews" ON reviews;
DROP POLICY IF EXISTS "Customers can update own reviews" ON reviews;

-- Create correct policies
CREATE POLICY "Customers can create reviews" ON reviews
  FOR INSERT WITH CHECK (customer_id = current_user_id());

CREATE POLICY "Customers can update own reviews" ON reviews
  FOR UPDATE USING (customer_id = current_user_id());

-- ============================================
-- FIX: Favorites policies - use customer_id NOT user_id
-- ============================================

-- Drop old policies (created dengan field salah)
DROP POLICY IF EXISTS "Users can read own favorites" ON favorites;
DROP POLICY IF EXISTS "Users can add favorites" ON favorites;
DROP POLICY IF EXISTS "Users can remove favorites" ON favorites;

-- Create correct policies
CREATE POLICY "Customers can read own favorites" ON favorites
  FOR SELECT USING (customer_id = current_user_id());

CREATE POLICY "Customers can add favorites" ON favorites
  FOR INSERT WITH CHECK (customer_id = current_user_id());

CREATE POLICY "Customers can remove favorites" ON favorites
  FOR DELETE USING (customer_id = current_user_id());

-- ============================================
-- VERIFICATION QUERIES
-- ============================================

-- Check all policies created
SELECT tablename, policyname
FROM pg_policies
WHERE schemaname = 'public'
  AND tablename IN ('reviews', 'favorites')
ORDER BY tablename, policyname;
