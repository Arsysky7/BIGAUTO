# Vehicle Service API - Testing Guide untuk Frontend

**Last Updated:** 2025-11-11
**Service Version:** 1.0.0
**Status:** ✅ PRODUCTION READY

---

## 🎯 QUICK START

### **Akses Dokumentasi Interaktif:**

1. **Swagger UI:** http://localhost:3003/swagger-ui
   - Testing interaktif dengan "Try it out" button
   - Semua endpoint bisa dicoba langsung
   - Auto-generated examples untuk semua request

2. **ReDoc:** http://localhost:3003/redoc
   - Documentation yang lebih clean
   - Cocok untuk dibaca & di-screenshot

3. **OpenAPI JSON:** http://localhost:3003/api-docs/openapi.json
   - Raw OpenAPI spec
   - Bisa diimport ke Postman/Insomnia

---

## 📊 ENDPOINT SUMMARY

| Category | Endpoint | Method | Auth Required |
|----------|----------|--------|---------------|
| **Vehicles** | `/api/vehicles` | GET | ❌ No |
| | `/api/vehicles` | POST | ✅ Yes (Seller) |
| | `/api/vehicles/{id}` | GET | ❌ No |
| | `/api/vehicles/{id}` | PUT | ✅ Yes (Seller) |
| | `/api/vehicles/{id}` | DELETE | ✅ Yes (Seller) |
| **Photos** | `/api/vehicles/{id}/photos` | POST | ✅ Yes (Seller) |
| | `/api/vehicles/{id}/photos/{index}` | DELETE | ✅ Yes (Seller) |
| **Filters** | `/api/filters/cities` | GET | ❌ No |
| | `/api/filters/brands` | GET | ❌ No |
| | `/api/filters/models` | GET | ❌ No |

**Total Endpoints:** 10
**Public Endpoints:** 5
**Seller-Only Endpoints:** 5

---

## 🔐 AUTHENTICATION

### **Cara Pakai JWT Token di Swagger UI:**

1. Klik tombol **"Authorize"** (🔓 icon) di pojok kanan atas
2. Masukkan JWT token dari auth-service:
   ```
   Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
   ```
3. Klik "Authorize"
4. Sekarang bisa test endpoint yang butuh authentication!

### **Cara Dapetin JWT Token:**

```bash
# 1. Login dulu di auth-service
curl -X POST http://localhost:3001/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "seller@example.com", "password": "password123"}'

# 2. Verifikasi OTP
curl -X POST http://localhost:3001/api/auth/verify-otp \
  -H "Content-Type: application/json" \
  -d '{"email": "seller@example.com", "otp": "123456"}'

# 3. Copy access_token dari response
# 4. Paste ke Swagger UI authorize dialog
```

---

## 🧪 TESTING SCENARIOS

### **Scenario 1: Guest User Browse Vehicles**

**Step 1:** List semua vehicles (NO AUTH NEEDED)
```bash
GET /api/vehicles?category=rental&city=Jakarta&page=1&limit=20
```

**Swagger UI Steps:**
1. Buka endpoint `GET /api/vehicles`
2. Klik "Try it out"
3. Isi filter parameters (optional):
   - category: `rental`
   - city: `Jakarta`
   - page: `1`
   - limit: `20`
4. Klik "Execute"
5. Lihat response di bawah!

**Expected Response:**
```json
{
  "data": [
    {
      "id": 1,
      "title": "Toyota Avanza 2022",
      "category": "rental",
      "price": 350000.0,
      "brand": "Toyota",
      "model": "Avanza",
      "year": 2022,
      "city": "Jakarta",
      "seller_name": "John Doe Auto Rent",
      "photos": ["https://..."],
      "status": "available",
      "rating": 4.8,
      "review_count": 25
    }
  ],
  "total": 50,
  "page": 1,
  "limit": 20,
  "total_pages": 3
}
```

---

**Step 2:** Get detail vehicle (NO AUTH NEEDED)
```bash
GET /api/vehicles/1
```

**Swagger UI Steps:**
1. Buka endpoint `GET /api/vehicles/{id}`
2. Klik "Try it out"
3. Isi `id`: `1`
4. Klik "Execute"

**Expected Response:**
```json
{
  "id": 1,
  "seller_id": 5,
  "seller_name": "John Doe Auto Rent",
  "title": "Toyota Avanza 2022 - Nyaman & Irit",
  "category": "rental",
  "price": 350000.0,
  "brand": "Toyota",
  "model": "Avanza",
  "year": 2022,
  "transmission": "Automatic",
  "fuel_type": "Bensin",
  "engine_capacity": 1500,
  "mileage": 15000,
  "seats": 7,
  "doors": 4,
  "luggage_capacity": 2,
  "vehicle_type": "MPV",
  "is_luxury": false,
  "is_flood_free": true,
  "tax_active": true,
  "has_bpkb": true,
  "has_stnk": true,
  "description": "Mobil keluarga yang nyaman dan irit...",
  "rental_terms": "- Wajib KTP\n- SIM A aktif...",
  "city": "Jakarta",
  "address": "Jl. Sudirman No. 123",
  "latitude": -6.208763,
  "longitude": 106.845599,
  "photos": [
    "https://res.cloudinary.com/.../car1.jpg",
    "https://res.cloudinary.com/.../car2.jpg"
  ],
  "status": "available",
  "rating": 4.8,
  "review_count": 25,
  "created_at": "2025-11-10T10:00:00Z",
  "updated_at": "2025-11-11T08:30:00Z"
}
```

---

### **Scenario 2: Seller Upload New Vehicle**

**Prerequisites:**
- ✅ JWT Token (seller role)
- ✅ Minimum 3 photos URL dari Cloudinary

**Step 1:** Create Vehicle (AUTH REQUIRED)
```bash
POST /api/vehicles
```

**Swagger UI Steps:**
1. **AUTHORIZE FIRST** (klik 🔓 icon, paste JWT token)
2. Buka endpoint `POST /api/vehicles`
3. Klik "Try it out"
4. Edit request body (example sudah auto-filled!):
   ```json
   {
     "title": "Toyota Avanza 2022 - Nyaman & Irit",
     "category": "rental",
     "price": 350000.0,
     "brand": "Toyota",
     "model": "Avanza",
     "year": 2022,
     "transmission": "Automatic",
     "fuel_type": "Bensin",
     "engine_capacity": 1500,
     "mileage": 15000,
     "seats": 7,
     "doors": 4,
     "luggage_capacity": 2,
     "vehicle_type": "MPV",
     "is_luxury": false,
     "is_flood_free": true,
     "tax_active": true,
     "has_bpkb": true,
     "has_stnk": true,
     "description": "Mobil keluarga yang nyaman dan irit",
     "rental_terms": "- Wajib KTP\n- SIM A aktif\n- Booking minimal 1 hari",
     "city": "Jakarta",
     "address": "Jl. Sudirman No. 123, Jakarta Pusat",
     "latitude": -6.208763,
     "longitude": 106.845599,
     "photos": [
       "https://res.cloudinary.com/demo/image/upload/v1/vehicles/car1.jpg",
       "https://res.cloudinary.com/demo/image/upload/v1/vehicles/car2.jpg",
       "https://res.cloudinary.com/demo/image/upload/v1/vehicles/car3.jpg"
     ]
   }
   ```
5. Klik "Execute"

**Success Response (201):**
```json
{
  "id": 123,
  "seller_id": 5,
  "seller_name": "John Doe",
  "title": "Toyota Avanza 2022 - Nyaman & Irit",
  "status": "available",
  ...
}
```

**Error Responses:**

❌ **401 Unauthorized** - No JWT token:
```json
{
  "error": "unauthorized",
  "message": "Missing authorization header"
}
```

❌ **403 Forbidden** - User bukan seller:
```json
{
  "error": "forbidden",
  "message": "Only sellers can create vehicles"
}
```

❌ **422 Validation Error** - Kurang dari 3 photos:
```json
{
  "error": "validation_error",
  "message": "Minimal 3 photos untuk category rental"
}
```

---

**Step 2:** Update Vehicle (AUTH REQUIRED)
```bash
PUT /api/vehicles/123
```

**Swagger UI Steps:**
1. Buka endpoint `PUT /api/vehicles/{id}`
2. Klik "Try it out"
3. Isi `id`: `123`
4. Edit request body (hanya field yang mau diubah):
   ```json
   {
     "title": "Toyota Avanza 2022 - Updated Title",
     "price": 320000.0,
     "mileage": 16000
   }
   ```
5. Klik "Execute"

**Success Response (200):**
```json
{
  "id": 123,
  "title": "Toyota Avanza 2022 - Updated Title",
  "price": 320000.0,
  "mileage": 16000,
  ...
}
```

**Error:**

❌ **403 Forbidden** - Bukan pemilik vehicle:
```json
{
  "error": "forbidden",
  "message": "You don't own this vehicle"
}
```

---

**Step 3:** Delete Vehicle (AUTH REQUIRED)
```bash
DELETE /api/vehicles/123
```

**Swagger UI Steps:**
1. Buka endpoint `DELETE /api/vehicles/{id}`
2. Klik "Try it out"
3. Isi `id`: `123`
4. Klik "Execute"

**Success Response (200):**
```json
{
  "message": "Vehicle berhasil dihapus"
}
```

---

### **Scenario 3: Seller Upload Additional Photos**

**Prerequisites:**
- ✅ JWT Token (seller role)
- ✅ Vehicle already exists
- ✅ Photo file (max 5MB, jpg/png)

**Endpoint:**
```bash
POST /api/vehicles/{id}/photos
```

**Swagger UI Steps:**
1. Buka endpoint `POST /api/vehicles/{id}/photos`
2. Klik "Try it out"
3. Isi `id`: `123`
4. Klik "Choose File" dan pilih gambar
5. Klik "Execute"

**Success Response (200):**
```json
{
  "id": 123,
  "photos": [
    "https://res.cloudinary.com/.../car1.jpg",
    "https://res.cloudinary.com/.../car2.jpg",
    "https://res.cloudinary.com/.../car3.jpg",
    "https://res.cloudinary.com/.../car4.jpg"
  ],
  ...
}
```

**Errors:**

❌ **422 Validation Error** - Maksimal 10 photos:
```json
{
  "error": "validation_error",
  "message": "Maksimal 10 photos per vehicle"
}
```

❌ **422 Validation Error** - File terlalu besar:
```json
{
  "error": "validation_error",
  "message": "File maksimal 5MB"
}
```

---

### **Scenario 4: Advanced Filtering**

**Test Filters:**

1. **Filter by Category + City:**
   ```
   GET /api/vehicles?category=rental&city=Jakarta
   ```

2. **Filter by Brand + Model:**
   ```
   GET /api/vehicles?brand=Toyota&model=Avanza
   ```

3. **Filter by Price Range:**
   ```
   GET /api/vehicles?min_price=200000&max_price=500000
   ```

4. **Filter by Year Range:**
   ```
   GET /api/vehicles?min_year=2020&max_year=2024
   ```

5. **Filter Luxury Cars Only:**
   ```
   GET /api/vehicles?is_luxury=true
   ```

6. **Sort by Price (Ascending):**
   ```
   GET /api/vehicles?sort_by=price_asc
   ```

7. **Sort by Price (Descending):**
   ```
   GET /api/vehicles?sort_by=price_desc
   ```

8. **Sort by Year (Newest First):**
   ```
   GET /api/vehicles?sort_by=year_desc
   ```

9. **Sort by Rating (Highest First):**
   ```
   GET /api/vehicles?sort_by=rating_desc
   ```

10. **Kombinasi Multiple Filters:**
    ```
    GET /api/vehicles?category=rental&city=Jakarta&brand=Toyota&min_price=300000&max_price=500000&min_year=2020&sort_by=price_asc&page=1&limit=10
    ```

**Swagger UI:** Semua filter parameters bisa diisi di "Try it out" form!

---

### **Scenario 5: Master Data (Cities, Brands, Models)**

**Get All Cities:**
```bash
GET /api/filters/cities
```

**Response:**
```json
[
  { "id": 1, "name": "Jakarta" },
  { "id": 2, "name": "Bandung" },
  { "id": 3, "name": "Surabaya" }
]
```

---

**Get All Brands:**
```bash
GET /api/filters/brands
```

**Response:**
```json
[
  { "id": 1, "name": "Toyota" },
  { "id": 2, "name": "Honda" },
  { "id": 3, "name": "Suzuki" }
]
```

---

**Get Models by Brand:**
```bash
GET /api/filters/models?brand=Toyota
```

**Response:**
```json
[
  { "id": 1, "brand_id": 1, "name": "Avanza" },
  { "id": 2, "brand_id": 1, "name": "Innova" },
  { "id": 3, "brand_id": 1, "name": "Fortuner" }
]
```

---

## ✅ VALIDATION RULES

### **Create Vehicle (POST /api/vehicles):**

| Field | Rules | Example |
|-------|-------|---------|
| title | 3-200 characters | ✅ "Toyota Avanza 2022" |
| category | "rental" or "sale" | ✅ "rental" |
| price | 1 - 10,000,000,000 | ✅ 350000 |
| year | 1900 - current year | ✅ 2022 |
| photos | Min 3 (rental), Min 5 (sale) | ✅ ["url1", "url2", "url3"] |
| city | Must exist in cities table | ✅ "Jakarta" |
| brand | Must exist in brands table | ✅ "Toyota" |

**Production Mode (RUST_ENV=production):**
- Validation lebih ketat
- File size limits lebih kecil
- More monitoring logs

---

## 🚨 COMMON ERROR RESPONSES

### **400 Bad Request:**
```json
{
  "error": "bad_request",
  "message": "Invalid request body"
}
```

### **401 Unauthorized:**
```json
{
  "error": "unauthorized",
  "message": "Missing authorization header"
}
```

### **403 Forbidden:**
```json
{
  "error": "forbidden",
  "message": "You don't own this vehicle"
}
```

### **404 Not Found:**
```json
{
  "error": "not_found",
  "message": "Vehicle tidak ditemukan"
}
```

### **422 Validation Error:**
```json
{
  "error": "validation_error",
  "message": "Price tidak valid (harus 1-10M)"
}
```

### **500 Internal Server Error:**
```json
{
  "error": "internal_server_error",
  "message": "Database error"
}
```

---

## 📝 TESTING CHECKLIST

### **Guest User (No Auth):**
- [ ] List vehicles tanpa filter
- [ ] List vehicles dengan filter category
- [ ] List vehicles dengan filter city
- [ ] List vehicles dengan filter price range
- [ ] List vehicles dengan filter year range
- [ ] List vehicles dengan sorting
- [ ] List vehicles dengan pagination
- [ ] Get vehicle detail by ID
- [ ] Get cities master data
- [ ] Get brands master data
- [ ] Get models by brand

### **Seller User (With Auth):**
- [ ] Create vehicle (rental category, 3 photos)
- [ ] Create vehicle (sale category, 5 photos)
- [ ] Create vehicle dengan validation errors (test each rule)
- [ ] Update vehicle (own vehicle)
- [ ] Update vehicle (not own - expect 403)
- [ ] Delete vehicle (own vehicle)
- [ ] Delete vehicle (not own - expect 403)
- [ ] Upload additional photos
- [ ] Upload photos (max limit - expect 422)
- [ ] Upload large file (>5MB - expect 422)
- [ ] Delete photo by index

### **Edge Cases:**
- [ ] Create vehicle dengan XSS payload (should be sanitized)
- [ ] Create vehicle dengan SQL injection attempt (should be safe)
- [ ] List vehicles dengan invalid pagination (negative page)
- [ ] List vehicles dengan huge limit (>1000)
- [ ] Get vehicle dengan non-existent ID
- [ ] Get models without brand parameter (expect 422)

---

## 🔥 QUICK TEST COMMANDS (curl)

```bash
# 1. Health Check
curl http://localhost:3003/health

# 2. List Vehicles
curl "http://localhost:3003/api/vehicles?page=1&limit=5"

# 3. Get Vehicle Detail
curl http://localhost:3003/api/vehicles/1

# 4. Get Cities
curl http://localhost:3003/api/filters/cities

# 5. Get Brands
curl http://localhost:3003/api/filters/brands

# 6. Get Models (Toyota)
curl "http://localhost:3003/api/filters/models?brand=Toyota"

# 7. Create Vehicle (NEED JWT!)
curl -X POST http://localhost:3003/api/vehicles \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "title": "Toyota Avanza 2022",
    "category": "rental",
    "price": 350000,
    "brand": "Toyota",
    "model": "Avanza",
    "year": 2022,
    "transmission": "Automatic",
    "seats": 7,
    "vehicle_type": "MPV",
    "city": "Jakarta",
    "address": "Jl. Sudirman No. 123",
    "photos": [
      "https://example.com/photo1.jpg",
      "https://example.com/photo2.jpg",
      "https://example.com/photo3.jpg"
    ]
  }'
```

---

## 🎨 UI/UX NOTES FOR FRONTEND

### **Recommended Field Displays:**

1. **Vehicle List Card:**
   - ✅ Show: title, price, year, city, photos[0], rating, review_count
   - ❌ Don't show: description, rental_terms (too long)

2. **Vehicle Detail Page:**
   - ✅ Show ALL fields
   - ✅ Photo gallery (semua photos)
   - ✅ Map integration (latitude/longitude)
   - ✅ Seller info (seller_name, contact button)

3. **Filter UI:**
   - ✅ Dropdown: category, city, brand, model, transmission
   - ✅ Range slider: price, year
   - ✅ Checkbox: is_luxury
   - ✅ Sort dropdown: price_asc, price_desc, year_desc, rating_desc

---

## 🚀 PERFORMANCE TIPS

1. **Pagination:**
   - Always use `page` and `limit` parameters
   - Recommended limit: 20-50 items per page
   - Max limit: 100

2. **Image Loading:**
   - Use lazy loading untuk photos
   - Show thumbnail first, load full image on click
   - Cloudinary URLs bisa ditambah transformation (resize, quality)

3. **Filtering:**
   - Send only filters yang user pilih (jangan send empty params)
   - Cache master data (cities, brands, models) di frontend

---

## 📞 SUPPORT

**Jika Ada Masalah:**

1. Cek service status: `curl http://localhost:3003/health`
2. Cek logs: `tail -f /tmp/vehicle-service.log`
3. Verify JWT token valid (belum expired)
4. Pastikan user punya role "seller" untuk seller endpoints

**Contact:**
- Backend Team: #backend-support
- Dokumentasi: http://localhost:3003/swagger-ui
- Issue Tracker: GitHub Issues

---

## ✅ FINAL CHECKLIST

- [x] ✅ Semua 10 endpoints documented di Swagger
- [x] ✅ Example values untuk semua request bodies
- [x] ✅ Example values untuk semua query parameters
- [x] ✅ Security scheme (JWT Bearer) configured
- [x] ✅ Response schemas lengkap
- [x] ✅ Error responses documented
- [x] ✅ Service running & healthy
- [x] ✅ Database connected
- [x] ✅ Authentication working
- [x] ✅ Validation working
- [x] ✅ Production-ready!

**STATUS: ✅ READY FOR FRONTEND TESTING!** 🎉

---

**Generated:** 2025-11-11
**Vehicle Service Version:** 1.0.0
**Documentation:** http://localhost:3003/swagger-ui
