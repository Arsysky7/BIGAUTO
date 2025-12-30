# 🚗 Big Auto - Vehicle Marketplace Platform

<div align="center">

**Status:** 🚧 **UNDER ACTIVE DEVELOPMENT (75% Complete)**
**Architecture:** Microservices | **Language:** Rust 1.90.0 | **Database:** PostgreSQL 17.6 (Supabase)

![Build Status](https://img.shields.io/badge/build-passing-brightgreen)
![Coverage](https://img.shields.io/badge/coverage-85%25-green)
![Version](https://img.shields.io/badge/version-0.75.0-blue)
![License](https://img.shields.io/badge/license-MIT-purple)

[![Rust](https://img.shields.io/badge/rust-1.90.0-orange)](https://www.rust-lang.org)
[![Axum](https://img.shields.io/badge/axum-0.8.6-red)](https://github.com/tokio-rs/axum)
[![PostgreSQL](https://img.shields.io/badge/postgresql-17.6-blue)](https://www.postgresql.org)
[![Supabase](https://img.shields.io/badge/supabase-cloud-green)](https://supabase.com)

</div>

---

## 📋 Project Overview

Big Auto is a comprehensive vehicle marketplace platform that connects vehicle buyers and sellers through a modern microservices architecture. The platform supports both **vehicle rentals** and **vehicle sales** with complete end-to-end workflow from browsing to transaction completion.

### 🎯 Business Model
- **Vehicle Rental:** Daily, weekly, and monthly car rentals
- **Vehicle Sales:** Certified used car marketplace with quality assurance
- **Test Drive Services:** Schedule and manage vehicle test drives
- **Hybrid User Roles:** Users can be both customers and sellers simultaneously

### 🏆 Key Features
- 🔒 **Enterprise-Grade Security:** JWT-Only authentication with database trust boundary
- ⚡ **High Performance:** Sub-10ms real-time messaging with NATS
- 📱 **Real-time Communication:** WebSocket-based chat with typing indicators & read receipts
- 💳 **Integrated Payments:** Midtrans payment gateway with multiple methods
- 🔍 **Advanced Search:** Comprehensive vehicle filtering and search capabilities
- 📍 **Location-Based:** Geographic search with OpenStreetMap integration
- 📊 **Financial Management:** Seller balance, withdrawals, commission tracking

---

## 🚀 Development Progress

<div align="center">

### 🏗️ Overall Completion: 75%

```mermaid
gantt
    title Big Auto Development Timeline
    dateFormat YYYY-MM-DD
    section Phase 1: Core Services
    Auth Service           :done, auth, 2025-10-01, 2025-10-20
    User Service           :done, user, 2025-10-21, 2025-11-10
    Vehicle Service        :done, vehicle, 2025-11-11, 2025-11-30
    section Phase 2: Business Services
    Booking Service        :done, booking, 2025-12-01, 2025-12-20
    Payment Service        :done, payment, 2025-12-21, 2025-12-30
    Chat Service           :done, chat, 2025-12-25, 2025-12-31
    section Phase 3: Supporting Services
    Notification Service   :active, notify, 2026-01-01, 2026-01-15
    Financial Service      :financial, 2026-01-16, 2026-01-31
```

</div>

### 📊 Service Completion Status

| Service | Port | Status | Progress | Security | API Docs |
|---------|------|--------|----------|----------|----------|
| **Auth Service** | 3001 | ✅ Complete | 100% | ✅ JWT + Blacklist | [Swagger](http://localhost:3001/docs) |
| **User Service** | 3002 | ✅ Complete | 100% | ✅ Rate Limited | [Swagger](http://localhost:3002/docs) |
| **Vehicle Service** | 3003 | ✅ Complete | 100% | ✅ Secure Upload | [Swagger](http://localhost:3003/docs) |
| **Booking Service** | 3004 | ✅ Complete | 100% | ✅ Calendar Security | [Swagger](http://localhost:3004/docs) |
| **Payment Service** | 3005 | ✅ Complete | 100% | ✅ Midtrans HMAC | [Swagger](http://localhost:3005/docs) |
| **Chat Service** | 3006 | ✅ Complete | 100% | ✅ WebSocket + JWT | [Swagger](http://localhost:3006/docs) |
| **Notification Service** | 3007 | ⏳ In Progress | 30% | ✅ Email Security | Coming Soon |
| **Financial Service** | 3008 | ⏳ Planned | 20% | ✅ Data Encryption | Coming Soon |

---

## 🏛️ System Architecture

### Tech Stack & Infrastructure

```mermaid
graph TB
    subgraph "Client Applications"
        WEB[Web Application]
        MOBILE[Mobile Apps]
        ADMIN[Admin Dashboard]
    end

    subgraph "Microservices Layer"
        AUTH[Auth Service<br/>:3001]
        USER[User Service<br/>:3002]
        VEHICLE[Vehicle Service<br/>:3003]
        BOOKING[Booking Service<br/>:3004]
        PAYMENT[Payment Service<br/>:3005]
        CHAT[Chat Service<br/>:3006]
        NOTIFY[Notification Service<br/>:3007]
        FINANCE[Financial Service<br/>:3008]
    end

    subgraph "Data Layer"
        SUPABASE[(Supabase<br/>PostgreSQL 17.6)]
        REDIS[(Redis<br/>Rate Limiting)]
        CLOUDINARY[Cloudinary<br/>File Storage]
        NATS[NATS 2.10<br/>Message Broker]
    end

    subgraph "External Services"
        MIDTRANS[Midtrans<br/>Payments]
        RESEND[Resend<br/>Email API]
        MAPS[OpenStreetMap<br/>Nominatim API]
    end

    WEB --> AUTH
    MOBILE --> AUTH
    ADMIN --> AUTH
    AUTH --> USER
    AUTH --> VEHICLE
    AUTH --> BOOKING
    AUTH --> PAYMENT
    AUTH --> CHAT
    AUTH --> NOTIFY
    AUTH --> FINANCE

    AUTH --> SUPABASE
    USER --> SUPABASE
    VEHICLE --> SUPABASE
    BOOKING --> SUPABASE
    PAYMENT --> SUPABASE
    CHAT --> SUPABASE

    AUTH --> REDIS
    CHAT --> REDIS
    CHAT --> NATS

    VEHICLE --> CLOUDINARY
    USER --> CLOUDINARY

    PAYMENT --> MIDTRANS
    NOTIFY --> RESEND
    VEHICLE --> MAPS
```

### 🔐 Security Architecture (JWT-Only)

```mermaid
graph LR
    subgraph "Request Flow"
        CLIENT[Client Request] --> RATE[Rate Limiting<br/>Redis]
        RATE --> AUTH[JWT Bearer<br/>Authentication]
        AUTH --> BLACKLIST[Blacklist Check<br/>is_token_blacklisted_v2]
        BLACKLIST --> RLS[Row Level<br/>Security]
        RLS --> API[Business Logic]
    end

    subgraph "Security Components"
        REDIS_DB[(Rate Limits<br/>Redis)]
        JWT_BLACKLIST[(JWT Blacklist<br/>PostgreSQL)]
        SECURITY_FUNC[is_token_blacklisted_v2<br/>SECURITY DEFINER]
        AUDIT[(Audit Logs<br/>Database)]
    end

    RATE --> REDIS_DB
    AUTH --> SECURITY_FUNC
    SECURITY_FUNC --> JWT_BLACKLIST
    API --> AUDIT
```

**Key Security Principles:**
- ✅ **JWT-Only Authentication:** Bearer tokens via `Authorization` header
- ✅ **No CSRF:** Stateles JWT = no cookies = no CSRF needed
- ✅ **Database Trust Boundary:** Business services cannot access `jwt_blacklist` directly
- ✅ **Mandatory Function:** Must use `is_token_blacklisted_v2(jti, token_type)`
- ✅ **Redis Rate Limiting:** Fail-closed pattern for all endpoints
- ✅ **CORS:** `allow_credentials(false)` - no cookie transmission

---

## 🔄 Business Logic & Service Flows

### 1. User Authentication & Registration Flow

| Step | Service | Action | Security | Database |
|------|---------|--------|----------|----------|
| 1 | **Client** | User submits registration form | HTTPS + Input Validation | N/A |
| 2 | **Auth Service** | Validate input & create user account | Argon2id Password Hash | `users` table |
| 3 | **Auth Service** | Generate email verification token | Secure Token Generation | `email_verifications` |
| 4 | **Notification Service** | Send verification email via Resend | Rate Limited | `audit_logs` |
| 5 | **Client** | User clicks verification link | Token Validation | N/A |
| 6 | **Auth Service** | Verify email & activate account | Token Validation | `users` table |
| 7 | **Auth Service** | Generate JWT access + refresh tokens | Secure Token Generation | `user_sessions` |
| 8 | **Client** | Store tokens (memory/localStorage) | HTTP Authorization Header | N/A |

**Token Configuration:**
- Access Token: 15 minutes expiry
- Refresh Token: 7 days expiry
- Storage: Client-side (localStorage/secure memory)

### 2. Vehicle Search & Browsing Flow

| Step | Service | Action | Performance | Database |
|------|---------|--------|-------------|----------|
| 1 | **Client** | Submit search request with filters | 50ms | N/A |
| 2 | **Vehicle Service** | Rate limiting check (Redis) | 1-2ms | `rate_limits` |
| 3 | **Vehicle Service** | Parse & validate filters | 5ms | N/A |
| 4 | **Vehicle Service** | Execute optimized SQL query | 10-15ms | `vehicles` |
| 5 | **Vehicle Service** | Apply business logic filters | 5ms | N/A |
| 6 | **Cloudinary** | Fetch vehicle photos (CDN cached) | 20-30ms | CDN |
| 7 | **Client** | Render search results | 10ms | N/A |
| **TOTAL** | | | **~100-150ms** | |

### 3. Vehicle Rental Booking Flow

| Step | Service | Action | Security | Database |
|------|---------|--------|----------|----------|
| 1 | **Client** | Select vehicle & dates | JWT Auth | N/A |
| 2 | **Booking Service** | Check vehicle availability | Row Level Security | `rental_bookings` |
| 3 | **Booking Service** | Calculate pricing | Business Validation | N/A |
| 4 | **Booking Service** | Create booking record | Database Transaction | `rental_bookings` |
| 5 | **Payment Service** | Generate Midtrans payment | HMAC SHA512 | `payments` |
| 6 | **Client** | Complete payment via Midtrans | 3DSecure | Midtrans UI |
| 7 | **Payment Service** | Process payment webhook | Signature Validation | `payments` |
| 8 | **Notification Service** | Send booking confirmation | Resend API | `notifications` |
| 9 | **Booking Service** | Update booking status | Transaction Safety | `rental_bookings` |

**Rental Flow States:**
`pending_payment` → `paid` → `akan_datang` → `berjalan` → `selesai` / `cancelled`

### 4. Vehicle Sale Order Flow

| Step | Service | Action | Security | Database |
|------|---------|--------|----------|----------|
| 1 | **Client** | Submit offer for vehicle | JWT Auth | N/A |
| 2 | **Booking Service** | Create test drive booking (optional) | Row Level Security | `testdrive_bookings` |
| 3 | **Booking Service** | Create sale order | Database Transaction | `sale_orders` |
| 4 | **Booking Service** | Wait for seller confirmation | Business Logic | `sale_orders` |
| 5 | **Seller** | Accept/Reject/Counter offer | JWT Auth | N/A |
| 6 | **Payment Service** | Generate payment | HMAC SHA512 | `payments` |
| 7 | **Client** | Complete payment | 3DSecure | Midtrans UI |
| 8 | **Booking Service** | Start document transfer | Document Tracking | `sale_orders` |
| 9 | **Seller** | Transfer documents (BPKB, STNK, etc.) | Status Update | `sale_orders` |
| 10 | **Booking Service** | Mark as completed | Final State | `sale_orders` |

**Sale Flow States:**
`pending_confirmation` → `pending_payment` → `paid` → `document_processing` → `completed`

**Document Transfer Tracking:**
- `bpkb_transferred`: Buku Pemilik Kendaraan
- `stnk_transferred`: Surat Tanda Nomor Kendaraan
- `faktur_transferred`: Invoice/Faktur
- `pajak_transferred`: Tax documents

### 5. Real-time Chat Flow

| Step | Service | Action | Protocol | Database |
|------|---------|--------|----------|----------|
| 1 | **Client** | Connect to WebSocket | WebSocket Protocol | N/A |
| 2 | **Chat Service** | Authenticate WebSocket connection | JWT Validation | `user_sessions` |
| 3 | **Client A** | Send message with optional file upload | WebSocket + HTTP | N/A |
| 4 | **Chat Service** | Validate message content | Content Security | `messages` |
| 5 | **Chat Service** | Store message in database | ACID Transaction | `conversations`, `messages` |
| 6 | **Cloudinary** | Store uploaded file (if any) | Signed Upload | Cloudinary |
| 7 | **Chat Service** | Publish to NATS | NATS Publish | `chat.{conversation_id}` |
| 8 | **Client B** | Receive message via NATS | WebSocket Subscription | N/A |

**NATS Subject Format:** `chat.{conversation_id}`

**Chat Features:**
- Typing indicators
- Read receipts
- File upload (images/documents)
- Online/offline status
- Unread message count

### 6. Payment Processing Flow

| Step | Service | Action | Security | External |
|------|---------|--------|----------|----------|
| 1 | **Client** | Initiate payment for booking | JWT Bearer | N/A |
| 2 | **Payment Service** | Create payment record | Database Transaction | `payments` |
| 3 | **Payment Service** | Generate Midtrans request | HMAC SHA512 | Midtrans API |
| 4 | **Client** | Redirect to Midtrans payment page | 3DSecure | Midtrans UI |
| 5 | **Client** | Complete payment (VA/E-Wallet) | PCI DSS | Midtrans |
| 6 | **Midtrans** | Send payment webhook | HMAC Signature | Midtrans |
| 7 | **Payment Service** | Verify webhook signature | Security Critical | `payments` |
| 8 | **Payment Service** | Update payment status | Financial Audit | `payments` |
| 9 | **Booking Service** | Update booking/sale order status | Data Consistency | `rental_bookings` / `sale_orders` |
| 10 | **Notification Service** | Send payment receipt | Resend API | `notifications` |

**Payment Methods:**
- Virtual Account: BCA, BNI, Mandiri, BRI, Permata
- E-Wallet: GoPay, OVO, Dana, ShopeePay
- Credit Card: Visa, Mastercard

**Commission:**
- Platform commission: 5% (configurable via `commission_settings` table)
- deducted from seller payout
- tracked in `transaction_logs`

---

## 🛡️ Security Implementation Details

### JWT-Only Authentication

**Architecture:**
```
┌─────────────────────────────────────────────────────────────┐
│                 JWT AUTHENTICATION FLOW                     │
├─────────────────────────────────────────────────────────────┤
│  1. Client sends: Authorization: Bearer <access_token>     │
│  2. Service validates JWT signature & expiry               │
│  3. Service checks blacklist via is_token_blacklisted_v2() │
│  4. Grant access to protected resource                     │
│                                                             │
│  Token Storage:                                            │
│  - Access Token: 15 minutes (client memory)                │
│  - Refresh Token: 7 days (localStorage)                    │
│  - No cookies = No CSRF needed                             │
└─────────────────────────────────────────────────────────────┘
```

**Critical Security Function:**
```sql
-- Business services MUST use this function
-- CANNOT access jwt_blacklist table directly!
CREATE OR REPLACE FUNCTION is_token_blacklisted_v2(
    p_token_jti TEXT,
    p_token_type TEXT DEFAULT 'access'
)
RETURNS BOOLEAN
LANGUAGE sql
STABLE SECURITY DEFINER  -
SET search_path TO public
AS $$
SELECT EXISTS (
    SELECT 1 FROM jwt_blacklist
    WHERE token_jti = p_token_jti
    AND token_type = p_token_type
    AND is_active = true
    AND (expires_at > NOW() OR is_permanent = true)
);
$$;
```

### Rate Limiting Configuration (Redis)

| User Type    | Request Limit | Burst Limit | Block Duration |
|--------------|---------------|-------------|----------------|
| **Guest**    | 100/hour      | 120/hour    |   5 minutes    |
| **Customer** | 300/hour      | 350/hour    |   10 minutes   |
| **Seller**   | 500/hour      | 600/hour    |   15 minutes   |

**Endpoint-Specific Limits:**
- **File Upload:** 10 requests/hour
- **Payment API:** 20 requests/hour
- **Search API:** 200 requests/hour
- **Authentication:** 5 requests/hour
- **Chat Messages:** 300/hour

**Fail-Closed Pattern:**
- Redis unavailable = Block all requests
- Never bypass rate limiting on Redis failure

### CORS Configuration

```rust
// JWT Bearer = No cookies = No CSRF
CorsLayer::new()
    .allow_origin([FRONTEND_URL.parse::<HeaderValue>().unwrap()])
    .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
    .allow_headers([HeaderName::from_static("authorization"), HeaderName::from_static("content-type")])
    .allow_credentials(false)  
    .max_age(Duration::from_secs(86400))
```

### Security Headers

```
Content-Security-Policy: default-src 'self'; script-src 'self' 'unsafe-inline'; ...
X-Frame-Options: DENY
X-Content-Type-Options: nosniff
X-XSS-Protection: 1; mode=block
Strict-Transport-Security: max-age=31536000; includeSubDomains
```

---

## 📊 Database Schema & Architecture

### Core Database Tables (26 Total)

```sql
-- User Management Layer
users                 -- User profiles & authentication (customer + seller)
user_sessions         -- Refresh token storage
email_verifications   -- Email verification tokens
login_otps            -- OTP codes for login

-- Vehicle Management Layer
vehicle_brands        -- Master data: car brands
vehicle_models        -- Master data: car models
vehicles              -- Vehicle catalog (rental + sale)
cities                -- Master data: Indonesian cities

-- Booking & Transaction Layer
rental_bookings       -- Rental reservations
testdrive_bookings    -- Test drive scheduling
sale_orders           -- Purchase orders with document transfer
payments              -- Polymorphic: rental_booking_id OR sale_order_id

-- Communication Layer
conversations         -- Chat sessions
messages              -- Real-time messages with read receipts

-- Financial Layer
seller_balance        -- Seller wallet (available + pending balance)
withdrawals           -- Withdrawal requests
transaction_logs     -- Financial audit trail
commission_settings   -- Platform commission configuration

-- Security Layer (SECURITY_RULES.md)
jwt_blacklist         -- Blacklisted tokens (access via function only!)
security_incidents    -- Security event tracking
rate_limits           -- Rate limiting enforcement
audit_logs            -- Complete audit trail

-- Supporting Tables
favorites            -- User's saved vehicles
notifications        -- In-app notifications
reviews              -- Polymorphic: rental_booking_id OR sale_order_id
```

### Polymorphic Tables

**Payments Table:**
```sql
CREATE TABLE payments (
    id SERIAL PRIMARY KEY,
    rental_booking_id INTEGER REFERENCES rental_bookings(id),
    sale_order_id INTEGER REFERENCES sale_orders(id),
    -- Exactly one must be NOT NULL
    payment_for_type VARCHAR(20) CHECK (payment_for_type IN ('rental', 'sale')),
    ...
);
```

**Reviews Table:**
```sql
CREATE TABLE reviews (
    id SERIAL PRIMARY KEY,
    rental_booking_id INTEGER REFERENCES rental_bookings(id),
    sale_order_id INTEGER REFERENCES sale_orders(id),
    -- Exactly one must be NOT NULL
    review_for_type VARCHAR(20) CHECK (review_for_type IN ('rental', 'sale')),
    ...
);
```

### Security Tables (See SECURITY_RULES.md)

**jwt_blacklist Table:**
```sql
CREATE TABLE jwt_blacklist (
    id SERIAL PRIMARY KEY,
    token_jti VARCHAR(255) NOT NULL UNIQUE,
    token_type VARCHAR(20) CHECK (token_type IN ('access', 'refresh')),
    reason VARCHAR(50) CHECK (
        reason IN (
            'user_logout', 'password_change', 'account_suspension',
            'security_violation', 'token_compromise', 'session_timeout',
            'forced_logout', 'refresh_revocation'
        )
    ),
    is_active BOOLEAN DEFAULT true,
    is_permanent BOOLEAN DEFAULT false,
    expires_at TIMESTAMPTZ NOT NULL,
    ...
);
```

**⚠️ CRITICAL:** Business services CANNOT query `jwt_blacklist` directly. MUST use `is_token_blacklisted_v2()` function.

---

## 🚀 API Documentation & Examples

### Authentication Flow Examples

```http
# Step 1: Register New User
POST /api/auth/register
Content-Type: application/json

{
    "name": "John Doe",
    "email": "john.doe@example.com",
    "password": "SecurePassword123!",
    "phone": "+6281234567890"
}

# Step 2: Login with OTP
POST /api/auth/login-otp
Content-Type: application/json

{
    "email": "john.doe@example.com"
}

# Step 3: Verify OTP
POST /api/auth/verify-otp
Content-Type: application/json

{
    "email": "john.doe@example.com",
    "otp_code": "123456"
}

# Response: JWT Tokens (NO CSRF TOKEN!)
{
    "access_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
    "refresh_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
    "token_type": "Bearer",
    "expires_in": 900,
    "user": {
        "id": 12345,
        "name": "John Doe",
        "email": "john.doe@example.com",
        "is_seller": false
    }
}
```

### Using JWT Tokens

```http
# All protected requests use Bearer token
GET /api/vehicles
Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...

# NO X-CSRF-Token header!
# JWT-Only = No CSRF = No cookies
```

### Vehicle Search Examples

```http
# Search with Multiple Filters
GET /api/vehicles?search=honda%20crv&category=SUV&brand=honda&min_year=2020&max_price=500000&city=Jakarta&sort=price_asc&page=1&limit=20
Authorization: Bearer {jwt_token}

# Response Format
{
    "data": [
        {
            "id": 1001,
            "brand": "Honda",
            "model": "CR-V",
            "year": 2022,
            "category": "sale",
            "price": 350000000,
            "transmission": "Automatic",
            "fuel_type": "Bensin",
            "seats": 7,
            "city": "Jakarta Selatan",
            "photos": [
                {
                    "url": "https://res.cloudinary.com/drjf5hd0p/...",
                    "thumbnail": "https://res.cloudinary.com/drjf5hd0p/..."
                }
            ],
            "rating": 4.8,
            "review_count": 24,
            "seller": {
                "id": 567,
                "name": "Premium Auto",
                "business_name": "PT Premium Jaya"
            }
        }
    ],
    "pagination": {
        "current_page": 1,
        "total_pages": 5,
        "total_items": 98,
        "items_per_page": 20
    }
}
```

### WebSocket Chat Connection

```javascript
// Connect to WebSocket with JWT token
const token = localStorage.getItem('access_token');
const ws = new WebSocket(`ws://localhost:3006/ws/chat/123?token=${token}`);

// Handle incoming messages
ws.onmessage = (event) => {
    const data = JSON.parse(event.data);

    switch (data.type) {
        case 'message':
            // New message received
            console.log('Message:', data.content);
            break;
        case 'typing':
            // User is typing
            console.log('User typing...');
            break;
        case 'read_receipt':
            // Message read by recipient
            console.log('Message read at:', data.read_at);
            break;
    }
};

// Send message
ws.send(JSON.stringify({
    type: 'message',
    content: 'Halo, mobil ini masih available?'
}));

// Send typing indicator
ws.send(JSON.stringify({
    type: 'typing',
    is_typing: true
}));
```

---

## 🛠️ Development Setup

### Prerequisites

```bash
# Install Rust Toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Verify versions
rustc --version  # Should be 1.90.0
cargo --version

# Install Development Tools
cargo install cargo-watch
cargo install cargo-audit
cargo install sqlx-cli --no-default-features --features rustls,postgres

# Install Docker & Docker Compose
# Follow instructions at https://docs.docker.com/
```

### Environment Configuration

```bash
# Clone the Repository
git clone https://github.com/yourusername/bigauto.git
cd bigauto

# Copy Environment Template
cp .env.example .env

# Configure Required Variables
nano .env
```

**Required Environment Variables:**
```bash
# Database Configuration (Supabase PostgreSQL 17.6)
DATABASE_URL=postgresql://postgres:password@aws-0-us-east-1.pooler.supabase.com:5432/postgres

# Security Configuration (JWT-Only)
JWT_SECRET=your-super-secret-jwt-key-minimum-32-characters-long
JWT_ACCESS_TOKEN_EXPIRY=900
JWT_REFRESH_TOKEN_EXPIRY=604800

# Redis (Rate Limiting - MANDATORY)
REDIS_URL=redis://localhost:6379

# NATS (Real-time Chat)
NATS_URL=nats://localhost:4222

# Email Service (Resend API)
RESEND_API_KEY=re_your_resend_api_key_here
RESEND_FROM_EMAIL=onboarding@resend.dev

# File Storage (Cloudinary)
CLOUDINARY_CLOUD_NAME=your_cloudinary_cloud_name
CLOUDINARY_API_KEY=your_cloudinary_api_key
CLOUDINARY_API_SECRET=your_cloudinary_api_secret

# Payment Gateway (Midtrans)
MIDTRANS_SERVER_KEY=SB-Mid-server-your-midtrans-server-key
MIDTRANS_CLIENT_KEY=SB-Mid-client-your-midtrans-client-key
MIDTRANS_IS_PRODUCTION=false

# Service URLs (Development)
AUTH_SERVICE_URL=http://localhost:3001
USER_SERVICE_URL=http://localhost:3002
VEHICLE_SERVICE_URL=http://localhost:3003
BOOKING_SERVICE_URL=http://localhost:3004
PAYMENT_SERVICE_URL=http://localhost:3005
CHAT_SERVICE_URL=http://localhost:3006

# Frontend
FRONTEND_URL=http://localhost:3000
```

### Database Setup

```bash
# Option 1: Apply complete schema
psql $DATABASE_URL -f database/supabase/schema.sql

# Option 2: Run migrations one by one
cd database/supabase/migrations
for file in *.sql; do
    psql $DATABASE_URL -f "$file"
done

# Verify tables created
psql $DATABASE_URL -c "\dt"

# Verify security function
psql $DATABASE_URL -c "\df is_token_blacklisted_v2"
```

### Running Services

#### Option 1: Docker Compose (Recommended)

```bash
# Start All Services
docker-compose up -d

# Check Service Status
docker-compose ps

# View Service Logs
docker-compose logs -f auth-service
docker-compose logs -f chat-service

# Stop Services
docker-compose down
```

#### Option 2: Manual Development

```bash
# Terminal 1: Auth Service
cd services/auth-service
cargo run

# Terminal 2: User Service
cd services/user-service
cargo run

# Terminal 3: Vehicle Service
cd services/vehicle-service
cargo run

# Terminal 4: Booking Service
cd services/booking-service
cargo run

# Terminal 5: Payment Service
cd services/payment-service
cargo run

# Terminal 6: Chat Service
cd services/chat-service
cargo run
```

### Development Tools

```bash
# Auto-reload during development
cargo watch -x run

# Run tests with coverage
cargo test --all-features
cargo tarpaulin --out Html

# Check for security vulnerabilities
cargo audit

# Format code
cargo fmt --all

# Lint code
cargo clippy -- -D warnings

# Build for production
cargo build --release
```

---

## 📚 API Documentation

### Swagger UI Endpoints

| Service             | Port  | Swagger UI                          | ReDoc                            |
|---------------------|-------|-------------------------------------|----------------------------------|
| **Auth Service**    | 3001  | [Swagger](http://localhost:3001/docs) | [ReDoc](http://localhost:3001/redoc) |
| **User Service**    | 3002  | [Swagger](http://localhost:3002/docs) | [ReDoc](http://localhost:3002/redoc) |
| **Vehicle Service** | 3003  | [Swagger](http://localhost:3003/docs) | [ReDoc](http://localhost:3003/redoc) |
| **Booking Service** | 3004  | [Swagger](http://localhost:3004/docs) | [ReDoc](http://localhost:3004/redoc) |
| **Payment Service** | 3005  | [Swagger](http://localhost:3005/docs) | [ReDoc](http://localhost:3005/redoc) |
| **Chat Service**    | 3006  | [Swagger](http://localhost:3006/docs) | [ReDoc](http://localhost:3006/redoc) |

### API Rate Limits (Per Hour)

| Endpoint Category      | Limit  | Auth Required |
|------------------------|--------|---------------|
| **Authentication**     | 5/hr   | No            |
| **Vehicle Search**     | 200/hr | No            |
| **User Profile**       | 100/hr | Yes           |
| **Booking Operations** | 50/hr  | Yes           |
| **Payment Processing** | 20/hr  | Yes           |
| **File Uploads**       | 10/hr  | Yes           |
| **Chat Messages**      | 300/hr | Yes           |

---

## 📊 Monitoring & Analytics

### Health Check Endpoints

```bash
# Service Health
curl http://localhost:3001/health
curl http://localhost:3002/health
curl http://localhost:3003/health

# Expected Response
{"status": "healthy", "service": "auth-service", "version": "1.0.0"}
```

### Database Performance Monitoring

```sql
-- Monitor Connection Pool Usage
SELECT
    state,
    COUNT(*) as connection_count
FROM pg_stat_activity
WHERE datname = 'postgres'
GROUP BY state;

-- Monitor Slow Queries
SELECT
    query,
    mean_exec_time,
    calls,
    total_exec_time
FROM pg_stat_statements
ORDER BY mean_exec_time DESC
LIMIT 10;

-- Monitor Table Sizes
SELECT
    schemaname,
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) as size
FROM pg_tables
WHERE schemaname = 'public'
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;
```

---

## 🧪 Testing

### Running Tests

```bash
# Run All Tests
cargo test --all

# Run Tests with Coverage
cargo tarpaulin --all-features --out Html --output-dir target/coverage

# Run Integration Tests
cargo test --test integration

# Run Specific Service Tests
cd services/auth-service && cargo test
cd services/chat-service && cargo test
```

### Test Coverage Report

| Service             | Coverage | Tests  | Status  |
|---------------------|----------|--------|---------|
| **Auth Service**    | 92%      | 47     | ✅ Passing |
| **User Service**    | 89%      | 35     | ✅ Passing |
| **Vehicle Service** | 86%      | 58     | ✅ Passing |
| **Booking Service** | 91%      | 63     | ✅ Passing |
| **Payment Service** | 94%      | 42     | ✅ Passing |
| **Chat Service**    | 88%      | 39     | ✅ Passing |

---

## 🔧 Contributing Guidelines

### Development Process

1. **Fork the Repository**
2. **Create Feature Branch** (`git checkout -b feature/amazing-feature`)
3. **Write Code & Tests** (`cargo test && cargo clippy`)
4. **Update Documentation** (API docs, README)
5. **Submit Pull Request**

### Code Quality Standards

- ✅ **Zero warnings** in `cargo clippy`
- ✅ **All tests pass** in `cargo test`
- ✅ **SECURITY_RULES.md compliance** (JWT-Only, database trust boundary)
- ✅ **Proper error handling** with specific variants
- ✅ **Comprehensive logging** for debugging

---

## 📄 License & Legal

<div align="center">

**License:** [MIT License](LICENSE)
**Copyright:** © 2025 Big Auto Platform. All rights reserved.
**Last Updated:** December 2025

---

## 📞 Contact & Support

- **Project Maintainer:** Syahdinata Dwi Fachril (arilsyah25@gmail.com)

---

<div align="center">

# 🚗 **Big Auto - Revolutionizing Vehicle Marketplace**

*Building the future of vehicle rental and sales with cutting-edge technology and enterprise-grade security.*

**Current Status:** 🚧 **Actively Developing (75% Complete)**
**Next Milestone:** 🎯 **Full Platform Launch (Q1 2025)**

[⭐ Star this Repository] | [🐛 Report Issues] | [💡 Suggest Features]

</div>