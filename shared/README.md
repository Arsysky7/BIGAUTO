# Shared Library

Common utilities untuk semua services.

## Modules

### `utils::cloudinary`

Client untuk upload/delete files ke Cloudinary.

```rust
use shared::utils::cloudinary::CloudinaryClient;

let client = CloudinaryClient::new()?;

// Upload image
let result = client.upload_image(bytes, "profiles", Some("user-123")).await?;

// Optimized URL
let url = client.optimized_url(&result.public_id, Some(400), Some(300), Some("auto"));

// Thumbnail
let thumb = client.thumbnail_url(&result.public_id, Some(150));

// Delete
client.delete_image(&result.public_id).await?;
```

### `utils::jwt`

JWT token validation (NOT generation - that's in auth-service).

```rust
use shared::utils::jwt::{validate_token, extract_user_id, is_seller};

// Validate token
let claims = validate_token(token)?;
println!("User ID: {}", claims.user_id);

// Extract user_id
let user_id = extract_user_id(token)?;

// Check if seller
let is_seller = is_seller(token)?;

// Extract from header
let token = extract_bearer_token(auth_header);
```

### `utils::validation`

Common validators.

```rust
use shared::utils::validation::*;

// Email
assert!(is_valid_email("user@example.com"));

// Phone (Indonesian format)
assert!(is_valid_phone("081234567890"));
assert!(is_valid_phone("+628123456789"));

// KTP (16 digits)
assert!(is_valid_ktp("1234567890123456"));

// Password (min 8, huruf + angka)
assert!(is_strong_password("Password123"));

// Price (positif, max 10 miliar)
assert!(is_valid_price(5_000_000));

// Year (1900-current+1)
assert!(is_valid_year(2024));

// Rating (1-5)
assert!(is_valid_rating(5));

// Sanitize HTML
let safe = sanitize_html("<script>alert('xss')</script>");
```

### `utils::http_client`

Inter-service HTTP calls.

```rust
use shared::utils::http_client::{ServiceClient, get_user_profile, get_vehicle};

// Manual client
let client = ServiceClient::new("vehicle")?;
let vehicle: VehicleResponse = client.get("/api/vehicles/123", Some(token)).await?;

// Helper functions
let profile = get_user_profile(user_id, token).await?;
let vehicle = get_vehicle(vehicle_id).await?;

// Check availability
let available = check_vehicle_availability(vehicle_id, "2024-01-01", "2024-01-05").await?;
```

## Import

In service `Cargo.toml`:

```toml
[dependencies]
shared = { path = "../../shared" }
```

In service code:

```rust
use shared::utils::cloudinary::CloudinaryClient;
use shared::utils::jwt::validate_token;
use shared::utils::validation::is_valid_email;
use shared::utils::http_client::ServiceClient;
```

## Environment Variables

Required:

```env
# JWT (for token validation)
JWT_SECRET=your-secret-key

# Cloudinary
CLOUDINARY_CLOUD_NAME=your-cloud-name
CLOUDINARY_API_KEY=your-api-key
CLOUDINARY_API_SECRET=your-api-secret

# Service URLs (for inter-service calls)
AUTH_SERVICE_URL=http://localhost:3001
USER_SERVICE_URL=http://localhost:3002
VEHICLE_SERVICE_URL=http://localhost:3003
# ... etc
```

## Testing

```bash
# Run tests
cargo test -p shared

# Build
cargo build -p shared
```

## Features

- ✅ Cloudinary storage (images & documents)
- ✅ JWT validation
- ✅ Common validators (email, phone, KTP, price, etc)
- ✅ Inter-service HTTP client
- ✅ Enterprise-grade code quality
- ✅ Comprehensive tests
