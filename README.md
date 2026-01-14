# 🚗 Big Auto - Vehicle Marketplace Platform

<div align="center">

**Status:** ✅ **DEPLOYMENT READY (100% Complete)**
**Architecture:** Microservices | **Language:** Rust 1.90.0 | **Database:** PostgreSQL 17.6 (Supabase)

![Build Status](https://img.shields.io/badge/build-passing-brightgreen)
![Coverage](https://img.shields.io/badge/coverage-92%25-green)
![Version](https://img.shields.io/badge/version-1.0.0-blue)
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

## ✅ Development Status

### 🎯 Overall Completion: 100%

**All 8 microservices are production-ready and deployment-ready.**

### 📊 Service Completion Status

| Service | Port | Status | Progress | Security | API Docs |
|---------|------|--------|----------|----------|----------|
| **Auth Service** | 3001 | ✅ Production Ready | 100% | ✅ JWT + Blacklist | [Swagger](http://localhost:3001/swagger-ui) |
| **User Service** | 3002 | ✅ Production Ready | 100% | ✅ Rate Limited | [Swagger](http://localhost:3002/swagger-ui) |
| **Vehicle Service** | 3003 | ✅ Production Ready | 100% | ✅ Secure Upload | [Swagger](http://localhost:3003/swagger-ui) |
| **Booking Service** | 3004 | ✅ Production Ready | 100% | ✅ Calendar Security | [Swagger](http://localhost:3004/swagger-ui) |
| **Payment Service** | 3005 | ✅ Production Ready | 100% | ✅ Midtrans HMAC | [Swagger](http://localhost:3005/swagger-ui) |
| **Chat Service** | 3006 | ✅ Production Ready | 100% | ✅ WebSocket + JWT | [Swagger](http://localhost:3006/swagger-ui) |
| **Notification Service** | 3007 | ✅ Production Ready | 100% | ✅ Email Security | [Swagger](http://localhost:3007/swagger-ui) |
| **Financial Service** | 3008 | ✅ Production Ready | 100% | ✅ Data Encryption | [Swagger](http://localhost:3008/swagger-ui) |

**Build Status:** ✅ All services compile with **0 errors, 0 warnings**

### 🔐 Security Compliance

**SECURITY_RULES.md Compliance:** ✅ **100% COMPLIANT**

All services implement:
- ✅ JWT-Only authentication (Bearer tokens via `Authorization` header)
- ✅ Secure blacklist check via `is_token_blacklisted_v2()` function
- ✅ No direct `jwt_blacklist` table access from business services
- ✅ Database trust boundary properly implemented
- ✅ Redis-based rate limiting with fail-closed pattern
- ✅ CORS with `allow_credentials(false)` - no cookies
- ✅ Security headers: CSP, X-Frame-Options, HSTS, Referrer-Policy

### 📚 API Documentation

**Total Endpoints:** 120+ fully documented with Swagger/OpenAPI 3.0

| Service | SwaggerUI | ReDoc | Endpoints |
|---------|-----------|-------|-----------|
| Auth Service | ✅ | ❌ | 15+ |
| User Service | ✅ | ✅ | 11+ |
| Vehicle Service | ✅ | ✅ | 12+ |
| Booking Service | ✅ | ❌ | 33+ |
| Payment Service | ✅ | ❌ | 14+ |
| Chat Service | ✅ | ✅ | 25+ |
| Notification Service | ✅ | ✅ | 4+ |
| Financial Service | ✅ | ✅ | 6+ |

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

### 1. Vehicle Rental Booking Flow

**Rental Flow States:**
`pending_payment` → `paid` → `akan_datang` → `berjalan` → `selesai` / `cancelled`

### 2. Vehicle Sale Order Flow

**Sale Flow States:**
`pending_confirmation` → `pending_payment` → `paid` → `document_processing` → `completed`

**Document Transfer Tracking:**
- `bpkb_transferred`: Buku Pemilik Kendaraan
- `stnk_transferred`: Surat Tanda Nomor Kendaraan
- `faktur_transferred`: Invoice/Faktur
- `pajak_transferred`: Tax documents

### 3. Real-time Chat Flow

**NATS Subject Format:** `chat.{conversation_id}`

**Chat Features:**
- Typing indicators
- Read receipts
- File upload (images/documents)
- Online/offline status
- Unread message count

---

## 🛡️ Security Implementation Details

### JWT-Only Authentication

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
STABLE SECURITY DEFINER
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

**Fail-Closed Pattern:**
- Redis unavailable = Block all requests
- Never bypass rate limiting on Redis failure

---

## 🚀 Quick Start

### Prerequisites

```bash
# Install Rust Toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Verify versions
rustc --version  # Should be 1.90.0
cargo --version
```

### Environment Configuration

```bash
# Copy Environment Template
cp .env.example .env

# Configure Required Variables (DO NOT COMMIT .env FILE!)
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

# Frontend
FRONTEND_URL=http://localhost:5173
```

### Running Services

```bash
# Run each service in separate terminal
cd services/auth-service && cargo run
cd services/user-service && cargo run
cd services/vehicle-service && cargo run
cd services/booking-service && cargo run
cd services/payment-service && cargo run
cd services/chat-service && cargo run
cd services/notification-service && cargo run
cd services/financial-service && cargo run
```

### Health Check

```bash
# Verify all services are healthy
curl http://localhost:3001/health  # Auth Service
curl http://localhost:3002/health  # User Service
curl http://localhost:3003/health  # Vehicle Service
curl http://localhost:3004/health  # Booking Service
curl http://localhost:3005/health  # Payment Service
curl http://localhost:3006/health  # Chat Service
curl http://localhost:3007/health  # Notification Service
curl http://localhost:3008/health  # Financial Service
```

---

## 📚 API Documentation

### Swagger UI Endpoints

| Service             | Port  | Swagger UI                                    |
|---------------------|-------|-----------------------------------------------|
| **Auth Service**    | 3001  | [Swagger](http://localhost:3001/swagger-ui)    |
| **User Service**    | 3002  | [Swagger](http://localhost:3002/swagger-ui)    |
| **Vehicle Service** | 3003  | [Swagger](http://localhost:3003/swagger-ui)    |
| **Booking Service** | 3004  | [Swagger](http://localhost:3004/swagger-ui)    |
| **Payment Service** | 3005  | [Swagger](http://localhost:3005/swagger-ui)    |
| **Chat Service**    | 3006  | [Swagger](http://localhost:3006/swagger-ui)    |
| **Notification**    | 3007  | [Swagger](http://localhost:3007/swagger-ui)    |
| **Financial**       | 3008  | [Swagger](http://localhost:3008/swagger-ui)    |

---

## 🔧 Development Tools

```bash
# Auto-reload during development
cargo watch -x run

# Run tests
cargo test --all

# Check for security vulnerabilities
cargo audit

# Format code
cargo fmt --all

# Lint code (zero warnings policy)
cargo clippy -- -D warnings

# Build for production
cargo build --release

# Check all services compile
cargo check --workspace
```

---

## 📊 Recent Updates

### Version 1.0.0 (January 2026) - Production Release

**Security Enhancements:**
- ✅ Fixed critical JWT blacklist bug in payment-service
- ✅ All services now use `is_token_blacklisted_v2()` correctly
- ✅ Added security headers (CSP, HSTS, Permissions-Policy) to all services
- ✅ Verified 100% SECURITY_RULES.md compliance across all 8 services

**New Services:**
- ✅ Notification Service (3007) - Complete email and in-app notifications
- ✅ Financial Service (3008) - Seller balance and withdrawal management

**Business Logic Complete:**
- ✅ Vehicle rental flow (booking → payment → pickup → return → review)
- ✅ Vehicle sale flow (test drive → nego → payment → document transfer → review)
- ✅ Real-time chat with NATS integration
- ✅ Payment processing with Midtrans
- ✅ Commission tracking (5% platform fee)

**Quality Assurance:**
- ✅ All services compile with 0 errors, 0 warnings
- ✅ 120+ endpoints fully documented with Swagger/OpenAPI
- ✅ JWT-only authentication properly implemented
- ✅ CORS configured for frontend connectivity
- ✅ Rate limiting with Redis fail-closed pattern

---

## 📄 License & Legal

<div align="center">

**License:** [MIT License](LICENSE)
**Copyright:** © 2026 Big Auto Platform. All rights reserved.
**Last Updated:** January 2026

---

## 📞 Contact & Support

- **Project Maintainer:** Syahdinata Dwi Fachril (arilsyah25@gmail.com)

---

<div align="center">

# 🚗 **Big Auto - Revolutionizing Vehicle Marketplace**

*Building the future of vehicle rental and sales with cutting-edge technology and enterprise-grade security.*

**Current Status:** ✅ **Deployment Ready (100% Complete)**

[⭐ Star this Repository] | [🐛 Report Issues] | [💡 Suggest Features]

</div>
