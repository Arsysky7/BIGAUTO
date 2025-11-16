# 📚 USER SERVICE - API DOCUMENTATION FOR FRONTEND

**Base URL:** `http://localhost:3002/api`
**Swagger UI:** http://localhost:3002/swagger-ui
**ReDoc:** http://localhost:3002/redoc

---

## 🔐 AUTHENTICATION

Semua endpoints (kecuali public) membutuhkan JWT token dari auth-service.

**Header Format:**
```
Authorization: Bearer {access_token}
```

**Cara Dapat Token:**
1. Login via auth-service: `POST /api/auth/login`
2. Verify OTP: `POST /api/auth/verify-otp`
3. Response akan dapat `access_token` (15 menit) dan `refresh_token` (7 hari)
4. Gunakan `access_token` untuk semua request user-service

---

## 📋 ENDPOINTS

### 1. PROFILE MANAGEMENT

#### 1.1 Get My Profile
**Endpoint:** `GET /api/users/me`
**Auth:** Required ✅
**Description:** Ambil profile user yang sedang login

**Request Headers:**
```http
Authorization: Bearer {access_token}
```

**Response 200 OK:**
```json
{
  "id": 1,
  "email": "john.doe@example.com",
  "name": "John Doe",
  "phone": "081234567890",
  "email_verified": true,
  "is_seller": true,
  "address": "Jl. Sudirman No. 123",
  "city": "Jakarta",
  "profile_photo": "https://res.cloudinary.com/drjf5hd0p/image/upload/v1234/profiles/user-1.jpg",
  "business_name": "Auto Rental Jakarta",
  "created_at": "2025-01-01T00:00:00Z"
}
```

**Response 401 Unauthorized:**
```json
{
  "error": "unauthorized",
  "message": "Token tidak valid atau sudah expired"
}
```

---

#### 1.2 Get User Profile (Public)
**Endpoint:** `GET /api/users/{user_id}`
**Auth:** Not Required ❌
**Description:** Ambil profile user lain (public view, untuk lihat seller info)

**Path Parameters:**
- `user_id` (integer, required) - User ID yang mau dilihat

**Example Request:**
```http
GET /api/users/123
```

**Response 200 OK:** (sama seperti Get My Profile)

**Response 404 Not Found:**
```json
{
  "error": "not_found",
  "message": "User tidak ditemukan"
}
```

---

#### 1.3 Update Profile
**Endpoint:** `PUT /api/users/me`
**Auth:** Required ✅
**Description:** Update profile user (semua field optional)

**Request Headers:**
```http
Authorization: Bearer {access_token}
Content-Type: application/json
```

**Request Body:** (semua field optional)
```json
{
  "name": "John Doe Updated",
  "phone": "081234567890",
  "address": "Jl. Sudirman No. 456",
  "city": "Jakarta",
  "business_name": "Auto Rental Jakarta Premium"
}
```

**Response 200 OK:** (return updated UserProfile)

**Response 400 Bad Request:**
```json
{
  "error": "validation_error",
  "message": "Format nomor HP tidak valid"
}
```

**Validation Rules:**
- `phone`: Format 08xxx atau +628xxx
- `name`: Min 3 karakter
- `business_name`: Min 3 karakter (jika ada)

---

#### 1.4 Upgrade to Seller
**Endpoint:** `POST /api/users/me/upgrade-seller`
**Auth:** Required ✅
**Description:** Upgrade akun customer menjadi seller

**Request Headers:**
```http
Authorization: Bearer {access_token}
Content-Type: application/json
```

**Request Body:**
```json
{
  "business_name": "Auto Rental Jakarta"
}
```

**Response 200 OK:** (return updated UserProfile dengan is_seller = true)

**Response 400 Bad Request:**
```json
{
  "error": "bad_request",
  "message": "User sudah menjadi seller"
}
```

---

#### 1.5 Upload Profile Photo
**Endpoint:** `POST /api/users/me/upload-photo`
**Auth:** Required ✅
**Description:** Upload foto profile ke Cloudinary

**Request Headers:**
```http
Authorization: Bearer {access_token}
Content-Type: multipart/form-data
```

**Request Body (multipart/form-data):**
```
file: [binary image data]
```

**Form Field Name:** `file` atau `photo`

**Response 200 OK:**
```json
{
  "url": "https://res.cloudinary.com/drjf5hd0p/image/upload/v1234/profiles/user-1.jpg",
  "thumbnail": "https://res.cloudinary.com/drjf5hd0p/image/upload/c_thumb,w_150/v1234/profiles/user-1.jpg"
}
```

**Response 400 Bad Request:**
```json
{
  "error": "validation_error",
  "message": "Ukuran file maksimal 5MB"
}
```

**Constraints:**
- Max file size: 5MB
- Supported formats: JPG, PNG, WEBP
- Auto-optimized by Cloudinary
- Thumbnail generated (150px width)

---

### 2. FAVORITES

#### 2.1 Get My Favorites
**Endpoint:** `GET /api/users/me/favorites`
**Auth:** Required ✅
**Description:** Ambil semua vehicle yang di-favorite

**Request Headers:**
```http
Authorization: Bearer {access_token}
```

**Response 200 OK:**
```json
[
  {
    "id": 1,
    "vehicle_id": 123,
    "created_at": "2025-01-01T00:00:00Z",
    "vehicle_title": "Toyota Avanza 2023 - Automatic",
    "vehicle_price": 350000,
    "vehicle_photo": "https://res.cloudinary.com/drjf5hd0p/image/upload/v1234/vehicles/avanza.jpg",
    "vehicle_city": "Jakarta"
  }
]
```

**Note:** Response include vehicle info (fetched from vehicle-service)

---

#### 2.2 Add Favorite
**Endpoint:** `POST /api/users/me/favorites`
**Auth:** Required ✅
**Description:** Tambah vehicle ke favorites

**Request Headers:**
```http
Authorization: Bearer {access_token}
Content-Type: application/json
```

**Request Body:**
```json
{
  "vehicle_id": 123
}
```

**Response 200 OK:**
```json
{
  "id": 1,
  "customer_id": 1,
  "vehicle_id": 123,
  "created_at": "2025-01-01T00:00:00Z"
}
```

**Response 400 Bad Request:**
```json
{
  "error": "bad_request",
  "message": "Vehicle sudah ada di favorites"
}
```

**Response 404 Not Found:**
```json
{
  "error": "not_found",
  "message": "Vehicle tidak ditemukan"
}
```

---

#### 2.3 Remove Favorite
**Endpoint:** `DELETE /api/users/me/favorites/{vehicle_id}`
**Auth:** Required ✅
**Description:** Hapus vehicle dari favorites

**Path Parameters:**
- `vehicle_id` (integer, required) - Vehicle ID yang mau dihapus

**Request Headers:**
```http
Authorization: Bearer {access_token}
```

**Example Request:**
```http
DELETE /api/users/me/favorites/123
Authorization: Bearer abc123token
```

**Response 200 OK:**
```json
{
  "message": "Favorite berhasil dihapus"
}
```

**Response 404 Not Found:**
```json
{
  "error": "not_found",
  "message": "Favorite tidak ditemukan"
}
```

---

#### 2.4 Check Favorite Status
**Endpoint:** `GET /api/users/me/favorites/check/{vehicle_id}`
**Auth:** Required ✅
**Description:** Cek apakah vehicle sudah di-favorite atau belum

**Path Parameters:**
- `vehicle_id` (integer, required) - Vehicle ID yang mau dicek

**Request Headers:**
```http
Authorization: Bearer {access_token}
```

**Example Request:**
```http
GET /api/users/me/favorites/check/123
Authorization: Bearer abc123token
```

**Response 200 OK:**
```json
{
  "is_favorite": true
}
```

**Use Case:** Untuk toggle button favorite di UI (show heart filled/unfilled)

---

### 3. RATINGS & REVIEWS

#### 3.1 Submit Review
**Endpoint:** `POST /api/sellers/{seller_id}/reviews`
**Auth:** Required ✅
**Description:** Submit review untuk seller setelah transaksi selesai

**Path Parameters:**
- `seller_id` (integer, required) - Seller ID yang mau direview

**Request Headers:**
```http
Authorization: Bearer {access_token}
Content-Type: application/json
```

**Request Body:**
```json
{
  "overall_rating": 5,
  "vehicle_condition_rating": 5,
  "accuracy_rating": 5,
  "service_rating": 5,
  "comment": "Pelayanan sangat baik, mobil bersih dan sesuai deskripsi!",
  "photos": [
    "https://res.cloudinary.com/drjf5hd0p/image/upload/v1234/reviews/review-1.jpg"
  ]
}
```

**Field Details:**
- `overall_rating` (integer, 1-5, **required**) - Rating keseluruhan
- `vehicle_condition_rating` (integer, 1-5, optional) - Rating kondisi mobil
- `accuracy_rating` (integer, 1-5, optional) - Rating sesuai deskripsi (untuk sale)
- `service_rating` (integer, 1-5, optional) - Rating pelayanan seller
- `comment` (string, optional, max 500 char) - Komentar review
- `photos` (array of strings, optional, max 3) - Foto review (Cloudinary URLs)

**Response 200 OK:**
```json
{
  "message": "Review berhasil disubmit",
  "review_id": 1
}
```

**Response 400 Bad Request:**
```json
{
  "error": "validation_error",
  "message": "Overall rating harus antara 1-5"
}
```

---

#### 3.2 Get Seller Ratings
**Endpoint:** `GET /api/sellers/{seller_id}/ratings`
**Auth:** Not Required ❌
**Description:** Ambil list reviews untuk seller (with pagination)

**Path Parameters:**
- `seller_id` (integer, required) - Seller ID

**Query Parameters:**
- `page` (integer, optional, default: 1) - Page number
- `limit` (integer, optional, default: 10) - Items per page

**Example Request:**
```http
GET /api/sellers/123/ratings?page=1&limit=10
```

**Response 200 OK:**
```json
[
  {
    "id": 1,
    "overall_rating": 5,
    "vehicle_condition_rating": 5,
    "accuracy_rating": 5,
    "service_rating": 5,
    "comment": "Pelayanan sangat baik, mobil bersih!",
    "photos": [
      "https://res.cloudinary.com/drjf5hd0p/image/upload/v1234/reviews/review-1.jpg"
    ],
    "created_at": "2025-01-01T00:00:00Z",
    "customer_id": 123,
    "customer_name": "John Doe",
    "customer_photo": "https://res.cloudinary.com/drjf5hd0p/image/upload/v1234/profiles/user-123.jpg"
  }
]
```

**Use Case:** Display reviews di halaman seller/vehicle detail

---

#### 3.3 Get Seller Rating Summary
**Endpoint:** `GET /api/sellers/{seller_id}/rating-summary`
**Auth:** Not Required ❌
**Description:** Ambil rating statistics untuk seller

**Path Parameters:**
- `seller_id` (integer, required) - Seller ID

**Example Request:**
```http
GET /api/sellers/123/rating-summary
```

**Response 200 OK:**
```json
{
  "seller_id": 1,
  "total_reviews": 150,
  "average_rating": 4.8,
  "rating_distribution": {
    "five_star": 120,
    "four_star": 20,
    "three_star": 5,
    "two_star": 3,
    "one_star": 2
  },
  "average_vehicle_condition": 4.9,
  "average_accuracy": 4.8,
  "average_service": 4.7
}
```

**Use Case:** Display rating summary di seller profile (e.g., "4.8 ⭐ (150 reviews)")

---

#### 3.4 Get My Seller Reviews (Seller Only)
**Endpoint:** `GET /api/sellers/me/reviews`
**Auth:** Required ✅ (Seller only)
**Description:** Seller melihat reviews yang diterima

**Request Headers:**
```http
Authorization: Bearer {access_token}
```

**Query Parameters:**
- `page` (integer, optional, default: 1)
- `limit` (integer, optional, default: 10)

**Response 200 OK:** (sama seperti Get Seller Ratings)

**Response 403 Forbidden:**
```json
{
  "error": "forbidden",
  "message": "Endpoint ini hanya untuk seller"
}
```

**Note:** Pastikan user sudah upgrade ke seller

---

## 🧪 TESTING DENGAN SWAGGER UI

1. **Start Service:**
   ```bash
   cd /home/arsy/Project/PPL/services/user-service
   cargo run
   ```

2. **Buka Swagger UI:**
   ```
   http://localhost:3002/swagger-ui
   ```

3. **Authorize (Untuk Endpoints yang Butuh Auth):**
   - Klik tombol **"Authorize" 🔒** di kanan atas
   - Masukkan token: `Bearer {access_token}`
   - Klik "Authorize"
   - Klik "Close"

4. **Test Endpoint:**
   - Expand endpoint yang mau di-test
   - Klik **"Try it out"**
   - Fill parameters/body (sudah ada example)
   - Klik **"Execute"**
   - Lihat response di bawah

---

## 📝 EXAMPLE FLOWS

### Flow 1: Upload Profile Photo
```bash
# 1. Login dulu (via auth-service)
POST /api/auth/login
{
  "email": "user@example.com",
  "password": "password123"
}

# 2. Verify OTP
POST /api/auth/verify-otp
{
  "email": "user@example.com",
  "otp": "123456"
}

# Response akan dapat access_token

# 3. Upload photo
POST /api/users/me/upload-photo
Headers: Authorization: Bearer {access_token}
Body: multipart/form-data
  file: [pilih gambar]

# Response:
{
  "url": "https://res.cloudinary.com/drjf5hd0p/image/upload/.../user-1.jpg",
  "thumbnail": "https://res.cloudinary.com/drjf5hd0p/.../user-1.jpg"
}
```

### Flow 2: Add to Favorites
```bash
# 1. User sudah login (punya access_token)

# 2. Add favorite
POST /api/users/me/favorites
Headers: Authorization: Bearer {access_token}
Body: {
  "vehicle_id": 123
}

# 3. Check favorite status (untuk UI toggle button)
GET /api/users/me/favorites/check/123
Headers: Authorization: Bearer {access_token}

# Response: { "is_favorite": true }
```

### Flow 3: Submit Review
```bash
# 1. User sudah selesai rental/beli mobil

# 2. Submit review
POST /api/sellers/456/reviews
Headers: Authorization: Bearer {access_token}
Body: {
  "overall_rating": 5,
  "vehicle_condition_rating": 5,
  "service_rating": 5,
  "comment": "Mobil bagus, pelayanan ramah!",
  "photos": []
}

# 3. Public bisa lihat review
GET /api/sellers/456/ratings?page=1&limit=10
# No auth needed
```

---

## ⚠️ COMMON ERRORS

### 401 Unauthorized
```json
{
  "error": "unauthorized",
  "message": "Token tidak valid atau sudah expired"
}
```
**Solution:** Login ulang atau refresh token

### 403 Forbidden
```json
{
  "error": "forbidden",
  "message": "Endpoint ini hanya untuk seller"
}
```
**Solution:** Upgrade to seller dulu via `POST /api/users/me/upgrade-seller`

### 404 Not Found
```json
{
  "error": "not_found",
  "message": "User tidak ditemukan"
}
```
**Solution:** Check user_id atau vehicle_id yang dikirim

### 400 Bad Request / Validation Error
```json
{
  "error": "validation_error",
  "message": "Format nomor HP tidak valid"
}
```
**Solution:** Check request body format

---

## 🔗 SERVICE DEPENDENCIES

User-service berkomunikasi dengan:
- **auth-service** - Untuk validasi JWT token
- **vehicle-service** - Untuk fetch vehicle info di favorites

Pastikan service-service tersebut juga running!

---

## 📞 SUPPORT

**Questions?** Contact backend team atau lihat:
- Swagger UI: http://localhost:3002/swagger-ui
- ReDoc: http://localhost:3002/redoc
- Source code: `/home/arsy/Project/PPL/services/user-service/`

---

**Last Updated:** 2025-11-09
**API Version:** 0.1.0
