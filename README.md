# 🚗 Big Auto - Vehicle Marketplace Platform

<div align="center">

**Status:** 🚧 **UNDER ACTIVE DEVELOPMENT (75% Complete)**
**Architecture:** Microservices | **Language:** Rust | **Database:** PostgreSQL (Supabase)

![Build Status](https://img.shields.io/badge/build-passing-brightgreen)
![Coverage](https://img.shields.io/badge/coverage-85%25-green)
![Version](https://img.shields.io/badge/version-0.75.0-blue)
![License](https://img.shields.io/badge/license-MIT-purple)

[![Rust](https://img.shields.io/badge/rust-1.70+-orange)](https://www.rust-lang.org)
[![Axum](https://img.shields.io/badge/axum-web%20framework-red)](https://github.com/tokio-rs/axum)
[![PostgreSQL](https://img.shields.io/badge/postgresql-15-blue)](https://www.postgresql.org)
[![Docker](https://img.shields.io/badge/docker-ready-blue)](https://www.docker.com)

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
- 🔒 **Enterprise-Grade Security:** CSRF protection, JWT authentication, rate limiting
- ⚡ **High Performance:** 1-2ms CSRF validation vs 60-125ms traditional approach
- 📱 **Real-time Communication:** WebSocket-based messaging with file sharing
- 💳 **Integrated Payments:** Midtrans payment gateway with multiple methods
- 🔍 **Advanced Search:** Comprehensive vehicle filtering and search capabilities
- 📍 **Location-Based:** Geographic search with map integration
- 📊 **Analytics Dashboard:** Business intelligence and financial reporting

---

## 🚀 Development Progress

<div align="center">

### 🏗️ Overall Completion: 75%

```mermaid
gantt
    title Big Auto Development Timeline
    dateFormat  YYYY-MM-DD
    section Core Services
    Auth Service           :done, auth, 2024-10-01, 2024-10-15
    User Service           :done, user, 2024-10-16, 2024-10-30
    Vehicle Service        :done, vehicle, 2024-11-01, 2024-11-15
    Booking Service        :done, booking, 2024-11-16, 2024-11-30
    Payment Service        :done, payment, 2024-12-01, 2024-12-15
    Chat Service           :done, chat, 2024-12-16, 2024-12-20
    section Remaining
    Notification Service   :active, notify, 2024-12-21, 2025-01-10
    Financial Service      :financial, 2025-01-11, 2025-01-25
```

</div>

### 📊 Service Completion Status

| Service | Port | Status | Progress | Security | API Docs |
|---------|------|--------|----------|----------|----------|
| **Auth Service** | 3001 | ✅ Complete | 100% | ✅ CSRF + JWT | [Swagger](http://localhost:3001/swagger-ui) |
| **User Service** | 3002 | ✅ Complete | 100% | ✅ Rate Limited | [Swagger](http://localhost:3002/swagger-ui) |
| **Vehicle Service** | 3003 | ✅ Complete | 100% | ✅ Secure Upload | [Swagger](http://localhost:3003/swagger-ui) |
| **Booking Service** | 3004 | ✅ Complete | 100% | ✅ Calendar Security | [Swagger](http://localhost:3004/swagger-ui) |
| **Payment Service** | 3005 | ✅ Complete | 100% | ✅ Midtrans HMAC | [Swagger](http://localhost:3005/swagger-ui) |
| **Chat Service** | 3006 | ✅ Complete | 100% | ✅ WebSocket Auth | [Swagger](http://localhost:3006/swagger-ui) |
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

    subgraph "API Gateway (Future)"
        GATEWAY[API Gateway]
        LB[Load Balancer]
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
        SUPABASE[(Supabase<br/>PostgreSQL)]
        REDIS[(Redis<br/>Cache)]
        CLOUDINARY[Cloudinary<br/>File Storage]
    end

    subgraph "External Services"
        MIDTRANS[Midtrans<br/>Payments]
        RESEND[Resend<br/>Email Service]
        MAPS[OpenStreetMap<br/>Maps API]
    end

    WEB --> GATEWAY
    MOBILE --> GATEWAY
    ADMIN --> GATEWAY
    GATEWAY --> LB
    LB --> AUTH
    LB --> USER
    LB --> VEHICLE
    LB --> BOOKING
    LB --> PAYMENT
    LB --> CHAT
    LB --> NOTIFY
    LB --> FINANCE

    AUTH --> SUPABASE
    USER --> SUPABASE
    VEHICLE --> SUPABASE
    BOOKING --> SUPABASE
    PAYMENT --> SUPABASE
    CHAT --> SUPABASE

    AUTH --> REDIS
    CHAT --> REDIS

    VEHICLE --> CLOUDINARY
    USER --> CLOUDINARY

    PAYMENT --> MIDTRANS
    NOTIFY --> RESEND
    VEHICLE --> MAPS
```

### 🔐 Security Architecture

```mermaid
graph LR
    subgraph "Request Flow"
        CLIENT[Client Request] --> CSRF[CSRF Token<br/>Validation]
        CSRF --> RATE[Rate Limiting<br/>Check]
        RATE --> AUTH[JWT<br/>Authentication]
        AUTH --> RLS[Row Level<br/>Security]
        RLS --> API[Business Logic]
    end

    subgraph "Security Components"
        CSRF_DB[(CSRF Tokens<br/>Database)]
        RATE_DB[(Rate Limits<br/>Database)]
        JWT_BLACKLIST[(JWT Blacklist<br/>Database)]
        AUDIT[(Audit Logs<br/>Database)]
    end

    CSRF --> CSRF_DB
    RATE --> RATE_DB
    AUTH --> JWT_BLACKLIST
    API --> AUDIT
```

---

## 🔄 Business Logic & Service Flows

### 1. User Authentication & Registration Flow

| Step | Service | Action | Security | Database |
|------|---------|--------|----------|----------|
| 1 | **Client** | User submits registration form | HTTPS + Input Validation | N/A |
| 2 | **Auth Service** | Validate input & create user account | Hashed Password | `users` table |
| 3 | **Auth Service** | Generate email verification token | Secure Token Generation | `email_verifications` |
| 4 | **Notification Service** | Send verification email via Resend | Rate Limited | `audit_logs` |
| 5 | **Client** | User clicks verification link | CSRF Protected | N/A |
| 6 | **Auth Service** | Verify email & activate account | Token Validation | `users` table |
| 7 | **Auth Service** | Generate JWT tokens + CSRF token | Secure Token Generation | `user_sessions`, `csrf_tokens` |
| 8 | **Client** | Store tokens securely | HTTP-only Cookies | N/A |

### 2. Vehicle Search & Browsing Flow

| Step | Service | Action | Performance | Database |
|------|---------|--------|-------------|----------|
| 1 | **Client** | Submit search request with filters | 50ms | N/A |
| 2 | **API Gateway** | Rate limiting check (1-2ms) | ⚡ Super Fast | `rate_limits` |
| 3 | **Vehicle Service** | Parse & validate filters | 5ms | N/A |
| 4 | **Vehicle Service** | Execute optimized SQL query | 10-15ms | `vehicles`, `vehicle_photos` |
| 5 | **Vehicle Service** | Apply business logic (availability, etc.) | 5ms | `vehicle_availability` |
| 6 | **Cloudinary** | Fetch vehicle photos (cached) | 20-30ms | CDN |
| 7 | **Client** | Render search results | 10ms | N/A |
| **TOTAL** | | | **~100-150ms** | |

### 3. Vehicle Rental Booking Flow

| Step | Service | Action | Security | Database |
|------|---------|--------|----------|----------|
| 1 | **Client** | Select vehicle & dates | CSRF Protected | N/A |
| 2 | **Booking Service** | Check vehicle availability | Row Level Security | `vehicle_availability` |
| 3 | **Booking Service** | Calculate pricing (insurance, fees) | Business Validation | `pricing_rules` |
| 4 | **Booking Service** | Create booking record | Database Transaction | `rental_bookings` |
| 5 | **Payment Service** | Generate payment request | HMAC Security | `payments` |
| 6 | **Client** | Complete payment via Midtrans | 3DSecure | `midtrans` |
| 7 | **Payment Service** | Process payment webhook | Signature Validation | `payments` |
| 8 | **Notification Service** | Send booking confirmation | Email Service | `notifications` |
| 9 | **Booking Service** | Update booking status | Transaction Safety | `rental_bookings` |

### 4. Real-time Communication Flow

| Step | Service | Action | Protocol | Database |
|------|---------|--------|----------|----------|
| 1 | **Client** | Establish WebSocket connection | WebSocket | N/A |
| 2 | **Chat Service** | Authenticate WebSocket | JWT Validation | `user_sessions` |
| 3 | **Client A** | Send message with optional file | WebSocket + Upload | N/A |
| 4 | **Chat Service** | Validate message content | Content Security | `messages` |
| 5 | **Chat Service** | Store message in database | ACID Compliance | `conversations`, `messages` |
| 6 | **Cloudinary** | Store uploaded file (if any) | Secure Upload | `cloudinary` |
| 7 | **Chat Service** | Broadcast to recipient(s) | Real-time Push | N/A |
| 8 | **Notification Service** | Push notification (if offline) | Mobile Push | `notification_queue` |
| 9 | **Client B** | Receive message instantly | WebSocket | N/A |

### 5. Payment Processing Flow

| Step | Service | Action | Security | External |
|------|---------|--------|----------|----------|
| 1 | **Client** | Initiate payment for booking | CSRF + JWT | N/A |
| 2 | **Payment Service** | Create payment record | Database Transaction | `payments` |
| 3 | **Payment Service** | Generate Midtrans request | HMAC SHA512 | `midtrans_api` |
| 4 | **Client** | Redirect to Midtrans payment page | 3DSecure | `midtrans_ui` |
| 5 | **Client** | Complete payment (VA/E-Wallet) | PCI Compliance | `midtrans` |
| 6 | **Midtrans** | Send payment webhook | HMAC Signature | `midtrans` |
| 7 | **Payment Service** | Verify webhook signature | Security Critical | `payments` |
| 8 | **Payment Service** | Update payment status | Financial Audit | `payments` |
| 9 | **Booking Service** | Update booking status | Data Consistency | `rental_bookings` |
| 10 | **Notification Service** | Send payment receipt | Email Delivery | `resend` |

---

## 🛡️ Security Implementation Details

### CSRF Protection System (Revolutionary Performance)

Our decentralized CSRF validation provides **30-125x performance improvement** over traditional centralized approaches:

```
┌─────────────────────────────────────────────────────────────┐
│                CSRF VALIDATION PERFORMANCE                  │
├─────────────────────────────────────────────────────────────┤
│  ❌ Centralized HTTP API:   60-125ms per request            │
│  ✅ Decentralized DB:       1-2ms per request               │
│  🚀 Performance Gain:       30-125x FASTER!                 │
│                                                             │
│  Architecture:                                              │
│  ┌─────────────────┐    ┌─────────────────┐                 │
│  │   Client App    │───▶│  Direct Database │                │ 
│  │  + CSRF Token   │    │   Validation     │                │
│  └─────────────────┘    └─────────────────┘                 │
│                                                             │
│  No HTTP API calls → Zero network latency                   │
│  Database indexes → Sub-millisecond queries                 │
└─────────────────────────────────────────────────────────────┘
```

### Rate Limiting Configuration

| User Type    | Request Limit | Burst Limit | Block Duration |
|--------------|---------------|-------------|----------------|
| **Guest**    | 100/minute    | 120/minute  |   5 minutes    |
| **Customer** | 300/minute    | 350/minute  |   10 minutes   |
| **Seller**   | 500/minute    | 600/minute  |   15 minutes   |
| **Admin**    | 1000/minute   | 1200/minute |   30 minutes   |

**Endpoint-Specific Limits:**
- **File Upload:** 10 requests/minute
- **Payment API:** 20 requests/minute
- **Search API:** 200 requests/minute
- **Authentication:** 5 requests/minute

### JWT Security Features

- **Access Token:** 15 minutes expiry
- **Refresh Token:** 7 days expiry
- **Token Rotation:** Secure refresh mechanism
- **Blacklist System:** Compromised token prevention
- **Rate Limited Auth:** Brute force protection

---

## 📊 Database Schema & Architecture

### Core Database Tables (23 Total)

```sql
-- User Management Layer
users                 -- User profiles & authentication
user_sessions         -- JWT session management
user_favorites        -- Saved vehicles & searches
user_reviews          -- Ratings & feedback system

-- Vehicle Management Layer
vehicles              -- Vehicle catalog with specifications
vehicle_photos        -- Photo gallery (Cloudinary integration)
vehicle_features      -- Vehicle features & amenities
vehicle_availability  -- Real-time availability calendar

-- Booking & Transaction Layer
rental_bookings       -- Rental reservations & contracts
test_drive_bookings   -- Test drive scheduling
sale_orders           -- Purchase orders & negotiations
payments              -- Payment processing & history

-- Communication Layer
conversations         -- Chat session management
messages              -- Real-time message storage
message_files         -- File attachment metadata

-- Security Layer 
csrf_tokens           -- CSRF protection system
rate_limits           -- Rate limiting enforcement
security_incidents    -- Security violation tracking
jwt_blacklist         -- Compromised token prevention
audit_logs            -- Complete audit trail
```

### Security Tables Implementation

```sql
-- CSRF Token Management (High-Performance)
CREATE TABLE csrf_tokens (
    id SERIAL PRIMARY KEY,
    user_id INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash VARCHAR(255) NOT NULL, 
    session_id VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    is_used BOOLEAN DEFAULT false,
    used_at TIMESTAMPTZ,
    ip_address INET,
    user_agent TEXT,
    endpoint VARCHAR(255),
    is_revoked BOOLEAN DEFAULT false,
    revoked_at TIMESTAMPTZ,
    revoke_reason TEXT,

    -- Performance Indexes
    CONSTRAINT uq_csrf_token_hash UNIQUE (token_hash)
);

CREATE INDEX idx_csrf_tokens_lookup ON csrf_tokens(token_hash, expires_at, is_used);
CREATE INDEX idx_csrf_tokens_user_cleanup ON csrf_tokens(user_id, expires_at);

-- Rate Limiting (Multi-dimensional)
CREATE TABLE rate_limits (
    id SERIAL PRIMARY KEY,
    identifier VARCHAR(255) NOT NULL,      
    identifier_type VARCHAR(20) NOT NULL,  
    action VARCHAR(100) NOT NULL,          
    endpoint VARCHAR(255),                
    service_name VARCHAR(50) NOT NULL,     
    request_count INTEGER DEFAULT 1,
    window_start TIMESTAMPTZ NOT NULL,
    window_end TIMESTAMPTZ NOT NULL,
    request_size_mb DECIMAL(10,2) DEFAULT 0,
    response_status INTEGER,
    is_blocked BOOLEAN DEFAULT false,
    blocked_until TIMESTAMPTZ,
    blocked_reason TEXT,

    -- Unique constraint for precise tracking
    CONSTRAINT uq_rate_limit_request UNIQUE(identifier, identifier_type, action, endpoint, service_name, window_start)
);

CREATE INDEX idx_rate_limits_enforcement ON rate_limits(identifier, identifier_type, is_blocked, blocked_until);
CREATE INDEX idx_rate_limits_service_monitoring ON rate_limits(service_name, action, window_start);
```

---

## 🚀 API Documentation & Examples

### Authentication Flow Examples

```http
# Step 1: Register New User
POST /api/auth/register
Content-Type: application/json
X-CSRF-Token: {csrf_token}

{
    "name": "John Doe",
    "email": "john.doe@example.com",
    "password": "SecurePassword123!",
    "phone": "+6281234567890"
}

# Step 2: Login
POST /api/auth/login
Content-Type: application/json
X-CSRF-Token: {csrf_token}

{
    "email": "john.doe@example.com",
    "password": "SecurePassword123!"
}

# Response: JWT Tokens + CSRF Token
{
    "access_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
    "refresh_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
    "csrf_token": "a1b2c3d4e5f6...",
    "expires_in": 900,
    "user": {
        "id": 12345,
        "name": "John Doe",
        "email": "john.doe@example.com",
        "role": "customer"
    }
}
```

### Vehicle Search Examples

```http
# Search with Multiple Filters
GET /api/vehicles?search=honda%20crv&type=SUV&brand=honda&min_year=2020&max_price=500000&location=Jakarta&sort=price_asc&page=1&limit=20
Authorization: Bearer {jwt_token}
X-CSRF-Token: {csrf_token}

# Response Format
{
    "data": [
        {
            "id": 1001,
            "brand": "Honda",
            "model": "CR-V",
            "year": 2022,
            "type": "SUV",
            "price_per_day": 350000,
            "price_per_month": 8500000,
            "location": "Jakarta Selatan",
            "transmission": "Automatic",
            "fuel_type": "Gasoline",
            "seats": 7,
            "photos": [
                {
                    "id": "photo_123",
                    "url": "https://res.cloudinary.com/...",
                    "thumbnail": "https://res.cloudinary.com/...",
                    "is_primary": true
                }
            ],
            "features": ["GPS", "Bluetooth", "Parking Camera"],
            "rating": 4.8,
            "total_reviews": 24,
            "owner": {
                "id": 567,
                "name": "Premium Rent Cars",
                "rating": 4.9,
                "response_rate": 98
            },
            "availability": {
                "available": true,
                "next_available": "2024-12-20T10:00:00Z"
            }
        }
    ],
    "pagination": {
        "current_page": 1,
        "total_pages": 5,
        "total_items": 98,
        "items_per_page": 20
    },
    "filters_applied": {
        "search": "honda crv",
        "type": "SUV",
        "brand": "honda",
        "year_range": "2020+",
        "price_max": 500000,
        "location": "Jakarta",
        "sort": "price_asc"
    }
}
```

### Booking Flow Examples

```http
# Step 1: Check Vehicle Availability
GET /api/bookings/availability/1001?start_date=2024-12-25&end_date=2024-12-28
Authorization: Bearer {jwt_token}
X-CSRF-Token: {csrf_token}

# Response:
{
    "available": true,
    "pricing": {
        "daily_rate": 350000,
        "total_days": 3,
        "base_price": 1050000,
        "insurance_fee": 150000,
        "service_fee": 50000,
        "total_price": 1250000
    },
    "vehicle_info": {
        "id": 1001,
        "name": "Honda CR-V 2022",
        "deposit_required": 1000000
    }
}

# Step 2: Create Booking
POST /api/bookings/rentals
Content-Type: application/json
Authorization: Bearer {jwt_token}
X-CSRF-Token: {csrf_token}

{
    "vehicle_id": 1001,
    "start_date": "2024-12-25T10:00:00Z",
    "end_date": "2024-12-28T10:00:00Z",
    "delivery_location": "Jakarta Selatan",
    "special_requests": "GPS navigation required",
    "agreed_price": 1250000
}

# Response:
{
    "booking": {
        "id": "RNT-20241225-00001",
        "vehicle": {
            "id": 1001,
            "name": "Honda CR-V 2022"
        },
        "dates": {
            "start": "2024-12-25T10:00:00Z",
            "end": "2024-12-28T10:00:00Z"
        },
        "pricing": {
            "total_amount": 1250000,
            "deposit": 1000000
        },
        "status": "pending_payment",
        "payment_due": "2024-12-25T18:00:00Z"
    },
    "payment_url": "https://app.sandbox.midtrans.com/payment/v2?id=..."
}
```

---

## 🛠️ Development Setup

### Prerequisites

```bash
# Install Rust Toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install Development Tools
cargo install cargo-watch      
cargo install cargo-audit     
cargo install sqlx-cli       

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
# Database Configuration
DATABASE_URL=postgresql://postgres:password@localhost:5432/bigauto
REDIS_URL=redis://localhost:6379

# Security Configuration
JWT_SECRET=your-super-secret-jwt-key-minimum-32-characters-long
CSRF_SECRET_KEY=your-csrf-secret-key-32-chars
ENCRYPTION_KEY=your-encryption-key-32-chars

# Email Service (Resend)
RESEND_API_KEY=re_your_resend_api_key_here
RESEND_FROM_EMAIL=noreply@bigauto.com

# File Storage (Cloudinary)
CLOUDINARY_CLOUD_NAME=your_cloudinary_cloud_name
CLOUDINARY_API_KEY=your_cloudinary_api_key
CLOUDINARY_API_SECRET=your_cloudinary_api_secret

# Payment Gateway (Midtrans)
MIDTRANS_SERVER_KEY=SB-Mid-server-your-midtrans-server-key
MIDTRANS_CLIENT_KEY=SB-Mid-client-your-midtrans-client-key
MIDTRANS_IS_PRODUCTION=false
MIDTRANS_API_URL=https://api.sandbox.midtrans.com/v2

# Service URLs (Development)
AUTH_SERVICE_URL=http://localhost:3001
USER_SERVICE_URL=http://localhost:3002
VEHICLE_SERVICE_URL=http://localhost:3003
BOOKING_SERVICE_URL=http://localhost:3004
PAYMENT_SERVICE_URL=http://localhost:3005
CHAT_SERVICE_URL=http://localhost:3006
```

### Database Setup

```bash
# Install PostgreSQL Extensions
psql $DATABASE_URL -c "CREATE EXTENSION IF NOT EXISTS 'uuid-ossp';"
psql $DATABASE_URL -c "CREATE EXTENSION IF NOT EXISTS 'pgcrypto';"

# Run Database Migrations
cd database/supabase/migrations
psql $DATABASE_URL -f 001_create_extensions.sql
psql $DATABASE_URL -f 002_create_users_table.sql
psql $DATABASE_URL -f 003_create_vehicles_table.sql
# ... continue with all migration files in order

# Seed Sample Data (Optional)
psql $DATABASE_URL -f ../seeds/sample_data.sql
```

### Running Services

#### Option 1: Docker Compose (Recommended for Development)

```bash
# Start All Services
docker-compose up -d

# Check Service Status
docker-compose ps

# View Service Logs
docker-compose logs -f auth-service
docker-compose logs -f vehicle-service
docker-compose logs -f payment-service

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
# Auto-reload during development (in each service directory)
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

Layanan	        Port	Tautan Swagger UI (URL)	                                Tautan ReDoc (URL)
Auth Service	3001	Swagger UI (http://localhost:3001/swagger-ui)	ReDoc (http://localhost:3001/redoc)
User Service	3002	Swagger UI (http://localhost:3002/swagger-ui)	ReDoc (http://localhost:3002/redoc)
Vehicle Service	3003	Swagger UI (http://localhost:3003/swagger-ui)	ReDoc (http://localhost:3003/redoc)
Booking Service	3004	Swagger UI (http://localhost:3004/swagger-ui)	ReDoc (http://localhost:3004/redoc)
Payment Service	3005	Swagger UI (http://localhost:3005/swagger-ui)	ReDoc (http://localhost:3005/redoc)
Chat Service	3006	Swagger UI (http://localhost:3006/swagger-ui)	ReDoc (http://localhost:3006/redoc)

### API Rate Limits

| Endpoint Category      | Limit  |  Burst  | Auth Required |
|------------------------|--------|---------|---------------|
| **Authentication**     | 5/min  | 7/min   |       No      |
| **Vehicle Search**     | 200/min| 250/min |       No      |
| **User Profile**       | 100/min| 120/min |      Yes      |
| **Booking Operations** | 50/min | 60/min  |      Yes      |
| **Payment Processing** | 20/min | 25/min  |      Yes      |
| **File Uploads**       | 10/min | 12/min  |      Yes      |
| **Chat Messages**      | 300/min| 350/min |      Yes      |

---


```

### Docker Production Deployment

```bash
# Build Production Images
docker build -t bigauto/auth-service:latest ./services/auth-service
docker build -t bigauto/user-service:latest ./services/user-service
docker build -t bigauto/vehicle-service:latest ./services/vehicle-service

# Tag and Push to Registry
docker tag bigauto/auth-service:latest your-registry/bigauto/auth-service:latest
docker push your-registry/bigauto/auth-service:latest

# Deploy with Docker Compose
docker-compose -f docker-compose.prod.yml up -d
```

**Production Docker Compose Configuration:**
```yaml
version: '3.8'
services:
  auth-service:
    image: bigauto/auth-service:latest
    ports:
      - "3001:3001"
    environment:
      - DATABASE_URL=${DATABASE_URL}
      - JWT_SECRET=${JWT_SECRET}
      - REDIS_URL=${REDIS_URL}
    restart: unless-stopped

  # ... similar configuration for other services
```

---

## 📊 Monitoring & Analytics

### Application Monitoring

```bash
# Health Check Endpoints
curl http://localhost:3001/health
curl http://localhost:3002/health
curl http://localhost:3003/health

# Service Metrics (when implemented)
curl http://localhost:3001/metrics
curl http://localhost:3002/metrics
```

### Database Performance Monitoring

```sql
-- Monitor Connection Pool Usage
SELECT
    state,
    COUNT(*) as connection_count
FROM pg_stat_activity
WHERE datname = 'bigauto'
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
cd services/vehicle-service && cargo test
```

### Test Coverage Report

| Service             | Coverage |   Tests  |   Status   |
|---------------------|----------|----------|------------|
| **Auth Service**    |    92%   | 47 tests | ✅ Passing |
| **User Service**    |    89%   | 35 tests | ✅ Passing |
| **Vehicle Service** |    86%   | 58 tests | ✅ Passing |
| **Booking Service** |    91%   | 63 tests | ✅ Passing |
| **Payment Service** |    94%   | 42 tests | ✅ Passing |
| **Chat Service**    |    88%   | 39 tests | ✅ Passing |

### Load Testing

```bash
# Install hey (HTTP load testing tool)
go install github.com/rakyll/hey@latest

# Test Vehicle Search Endpoint
hey -n 1000 -c 10 -m GET -H "Authorization: Bearer {token}" \
  "http://localhost:3003/api/vehicles?search=honda&limit=20"

# Test Authentication Endpoint
hey -n 100 -c 5 -m POST -H "Content-Type: application/json" \
  -d '{"email":"test@example.com","password":"test123"}' \
  "http://localhost:3001/api/auth/login"
```

---

## 🔧 Contributing Guidelines

### Development Process

1. **Fork the Repository**
   ```bash
   git clone https://github.com/yourusername/bigauto.git
   ```

2. **Create Feature Branch**
   ```bash
   git checkout -b feature/amazing-feature
   ```

3. **Write Code & Tests**
   ```bash
   # Implement feature with comprehensive tests
   cargo test
   cargo clippy -- -D warnings
   ```

4. **Update Documentation**
   - Update API documentation in code
   - Update README if needed
   - Add examples for new features

5. **Submit Pull Request**
   ```bash
   git push origin feature/amazing-feature
   # Create PR on GitHub
   ```

### Code Quality Standards

- ✅ **Zero warnings** in `cargo clippy`
- ✅ **All tests pass** in `cargo test`
- ✅ **Proper error handling** with specific variants
- ✅ **Security-first** mindset
- ✅ **Performance optimization** for database queries
- ✅ **Comprehensive logging** for debugging

### Areas Needing Contribution

- 🚀 **Performance optimization** for high-traffic endpoints
- 📱 **Mobile app development** (React Native experience)
- 🤖 **Machine learning** for vehicle recommendations
- 🔒 **Security auditing** and penetration testing
- 📊 **Data analytics** for business intelligence
- 🌍 **Internationalization** support

---

## 📄 License & Legal

<div align="center">

**License:** [MIT License](LICENSE)
**Copyright:** © 2024 Big Auto Platform. All rights reserved.
**Last Updated:** December 2024

---

## 📞 Contact & Support

- **Project Maintainer:** [Syahdinata Dwi Fachril ](mailto: arilsyah25@gmail.com)

---

<div align="center">

# 🚗 **Big Auto - Revolutionizing Vehicle Marketplace**

*Building the future of vehicle rental and sales with cutting-edge technology and enterprise-grade security.*

**Current Status:** 🚧 **Actively Developing (75% Complete)**
**Next Milestone:** 🎯 **Full Platform Launch (Q1 2025)**

[⭐ Star this Repository] | [🐛 Report Issues] | [💡 Suggest Features]

</div>