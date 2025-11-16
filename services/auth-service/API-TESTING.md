# Big Auto - Auth Service API Testing Guide

Base URL: `http://localhost:3001`

## Schemas Documentation

Buka Swagger UI untuk melihat semua schemas (request/response models):
**Swagger UI:** http://localhost:3001/swagger-ui

## API Endpoints

### 1. Health Check

**GET** `/health`

Test:
```bash
curl http://localhost:3001/health
```

---

### 2. Register (Daftar Akun Baru)

**POST** `/api/auth/register`

Request Body:
```json
{
  "email": "john@example.com",
  "password": "SecureP@ss123",
  "name": "John Doe",
  "phone": "+6281234567890",
  "address": "Jl. Sudirman No. 123",
  "city": "Jakarta"
}
```

Test:
```bash
curl -X POST http://localhost:3001/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "john@example.com",
    "password": "SecureP@ss123",
    "name": "John Doe",
    "phone": "+6281234567890",
    "address": "Jl. Sudirman No. 123",
    "city": "Jakarta"
  }'
```

Response (201 Created):
```json
{
  "user_id": 1,
  "email": "john@example.com",
  "name": "John Doe",
  "message": "Registrasi berhasil. Silakan cek email untuk verifikasi akun."
}
```

---

### 3. Verify Email

**GET** `/api/auth/verify-email?token={token}`

*(Token didapat dari email)*

Test:
```bash
curl "http://localhost:3001/api/auth/verify-email?token=abc123def456"
```

Response:
```json
{
  "message": "Email berhasil diverifikasi. Silakan login."
}
```

---

### 4. Resend Verification Email

**POST** `/api/auth/resend-verification`

Request Body:
```json
{
  "email": "john@example.com"
}
```

Test:
```bash
curl -X POST http://localhost:3001/api/auth/resend-verification \
  -H "Content-Type: application/json" \
  -d '{"email": "john@example.com"}'
```

---

### 5. Login Step 1 (Kirim OTP ke Email)

**POST** `/api/auth/login`

Request Body:
```json
{
  "email": "john@example.com",
  "password": "SecureP@ss123"
}
```

Test:
```bash
curl -X POST http://localhost:3001/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "john@example.com",
    "password": "SecureP@ss123"
  }'
```

Response:
```json
{
  "message": "OTP telah dikirim ke email Anda. Kode berlaku 5 menit.",
  "user_id": 1
}
```

---

### 6. Login Step 2 (Verifikasi OTP & Get JWT Tokens)

**POST** `/api/auth/verify-otp`

Request Body:
```json
{
  "user_id": 1,
  "otp_code": "123456"
}
```

Test:
```bash
curl -X POST http://localhost:3001/api/auth/verify-otp \
  -H "Content-Type: application/json" \
  -c cookies.txt \
  -d '{
    "user_id": 1,
    "otp_code": "123456"
  }'
```

Response:
```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIs...",
  "user": {
    "id": 1,
    "email": "john@example.com",
    "name": "John Doe",
    "phone": "+6281234567890",
    "is_seller": false,
    "email_verified": true
  },
  "message": "Login berhasil."
}
```

**Note:** Refresh token akan disimpan di httpOnly cookie automatically.

---

### 7. Resend OTP

**POST** `/api/auth/resend-otp`

Request Body:
```json
{
  "user_id": 1
}
```

Test:
```bash
curl -X POST http://localhost:3001/api/auth/resend-otp \
  -H "Content-Type: application/json" \
  -d '{"user_id": 1}'
```

---

### 8. Refresh Access Token

**POST** `/api/auth/refresh`

*(Menggunakan refresh token dari cookie)*

Test:
```bash
curl -X POST http://localhost:3001/api/auth/refresh \
  -b cookies.txt
```

Response:
```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIs..."
}
```

---

### 9. Logout

**POST** `/api/auth/logout`

Test:
```bash
curl -X POST http://localhost:3001/api/auth/logout \
  -b cookies.txt \
  -c cookies.txt
```

Response:
```json
{
  "message": "Logout berhasil."
}
```

---

### 10. Check OTP Status (Protected)

**GET** `/api/auth/otp-status`

Requires: JWT token in Authorization header

Test:
```bash
curl http://localhost:3001/api/auth/otp-status \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN"
```

Response:
```json
{
  "is_blocked": false,
  "blocked_until": null,
  "remaining_minutes": null,
  "message": "Akun Anda tidak diblokir. Anda dapat request OTP."
}
```

---

### 11. Get Active Sessions (Protected)

**GET** `/api/auth/sessions`

Requires: JWT token

Test:
```bash
curl http://localhost:3001/api/auth/sessions \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN"
```

Response:
```json
[
  {
    "id": 1,
    "user_id": 1,
    "device_name": "Chrome on Windows",
    "ip_address": "192.168.1.1",
    "last_activity": "2025-10-31T01:00:00Z",
    "expires_at": "2025-11-07T01:00:00Z",
    "is_current": true
  }
]
```

---

### 12. Invalidate Single Session (Protected)

**DELETE** `/api/auth/sessions/{id}`

Requires: JWT token

Test:
```bash
curl -X DELETE http://localhost:3001/api/auth/sessions/1 \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN"
```

Response:
```json
{
  "message": "Session berhasil dihapus. Device telah logout."
}
```

---

### 13. Invalidate All Sessions (Protected)

**POST** `/api/auth/sessions/invalidate-all`

Requires: JWT token

Test:
```bash
curl -X POST http://localhost:3001/api/auth/sessions/invalidate-all \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN"
```

Response:
```json
{
  "message": "Berhasil logout dari semua device."
}
```

---

### 14. Get User Profile (Protected)

**GET** `/api/users/me`

Requires: JWT token

Test:
```bash
curl http://localhost:3001/api/users/me \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN"
```

Response:
```json
{
  "id": 1,
  "email": "john@example.com",
  "name": "John Doe",
  "phone": "+6281234567890",
  "is_seller": false,
  "address": "Jl. Sudirman No. 123",
  "city": "Jakarta",
  "profile_photo": null,
  "business_name": null,
  "email_verified": true
}
```

---

### 15. Update User Profile (Protected)

**PUT** `/api/users/me`

Requires: JWT token

Request Body (semua field opsional):
```json
{
  "name": "John Doe Updated",
  "phone": "+6281234567890",
  "address": "Jl. Sudirman No. 456",
  "city": "Bandung",
  "profile_photo": "https://example.com/photos/profile.jpg"
}
```

Test:
```bash
curl -X PUT http://localhost:3001/api/users/me \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  -d '{
    "name": "John Doe Updated",
    "city": "Bandung"
  }'
```

Response: Same as Get Profile

---

### 16. Upgrade to Seller (Protected)

**POST** `/api/users/me/upgrade-seller`

Requires: JWT token

Request Body:
```json
{
  "business_name": "Toko Mobil Sejahtera"
}
```

Test:
```bash
curl -X POST http://localhost:3001/api/users/me/upgrade-seller \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  -d '{"business_name": "Toko Mobil Sejahtera"}'
```

Response: Profile dengan `is_seller: true`

---

## Testing Flow (Complete Journey)

### 1. Register & Login
```bash
# 1. Register
curl -X POST http://localhost:3001/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@example.com",
    "password": "TestPass123!",
    "name": "Test User",
    "phone": "+6281234567890"
  }'

# 2. Verify email (cek email untuk token)
curl "http://localhost:3001/api/auth/verify-email?token=YOUR_TOKEN"

# 3. Login Step 1
curl -X POST http://localhost:3001/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@example.com",
    "password": "TestPass123!"
  }'
# Save user_id from response

# 4. Login Step 2 (cek email untuk OTP)
curl -X POST http://localhost:3001/api/auth/verify-otp \
  -H "Content-Type: application/json" \
  -c cookies.txt \
  -d '{
    "user_id": 1,
    "otp_code": "YOUR_OTP"
  }'
# Save access_token from response
```

### 2. Use Protected Endpoints
```bash
# Set your token
TOKEN="YOUR_ACCESS_TOKEN_HERE"

# Get profile
curl http://localhost:3001/api/users/me \
  -H "Authorization: Bearer $TOKEN"

# Update profile
curl -X PUT http://localhost:3001/api/users/me \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"city": "Surabaya"}'

# Upgrade to seller
curl -X POST http://localhost:3001/api/users/me/upgrade-seller \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"business_name": "My Business"}'

# Get sessions
curl http://localhost:3001/api/auth/sessions \
  -H "Authorization: Bearer $TOKEN"
```

### 3. Logout
```bash
curl -X POST http://localhost:3001/api/auth/logout \
  -b cookies.txt
```

---

## Error Responses

All errors follow this format:
```json
{
  "error": "Error message here"
}
```

Common HTTP Status Codes:
- `200` - Success
- `201` - Created (register success)
- `400` - Bad Request (validation error)
- `401` - Unauthorized (invalid/expired token)
- `404` - Not Found
- `409` - Conflict (email already exists)
- `500` - Internal Server Error

---

## Additional Info

### Schemas Available in Swagger UI

Buka http://localhost:3001/swagger-ui untuk melihat:
- `RegisterRequestBody`
- `LoginRequestBody`
- `VerifyOtpRequestBody`
- `LoginStep1Response`
- `LoginStep2Response`
- `UserData`
- `ProfileResponse`
- `SessionResponse`
- `UpdateProfileRequestBody`
- `UpgradeToSellerRequestBody`
- `OtpStatusResponse`
- `MessageResponse`
- `RefreshTokenResponse`
- `HealthCheckResponse`

### Environment Variables

Pastikan `.env` sudah dikonfigurasi dengan benar:
```env
DATABASE_URL=postgresql://...
REDIS_URL=redis://localhost:6379
JWT_SECRET=your-secret-key
SMTP_HOST=smtp.gmail.com
SMTP_USERNAME=your-email
SMTP_PASSWORD=your-app-password
```
