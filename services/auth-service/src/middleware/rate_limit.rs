// Redis-based Rate Limiting untuk Auth Service 

use axum::{
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::Next,
    response::Response,
};
use serde::{Deserialize, Serialize};
use std::env;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Rate Limiter structure dengan Redis
#[derive(Clone)]
pub struct AuthRateLimiter {
    redis_client: redis::Client,
    window_seconds: u64,
}

/// Rate limit data structure untuk Redis storage
#[derive(Serialize, Deserialize, Debug)]
struct RateLimitData {
    count: u32,
    window_start: u64,
}

/// Environment variable constants untuk rate limits 
const RATE_LIMIT_GUEST: &str = "RATE_LIMIT_GUEST_REQUESTS";
const RATE_LIMIT_CUSTOMER: &str = "RATE_LIMIT_CUSTOMER_REQUESTS";
const RATE_LIMIT_SELLER: &str = "RATE_LIMIT_SELLER_REQUESTS";
const RATE_LIMIT_SENSITIVE: &str = "RATE_LIMIT_SENSITIVE_ENDPOINTS";

impl AuthRateLimiter {
    /// Create new Redis-based rate limiter
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let redis_url = env::var("REDIS_URL")
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "REDIS_URL environment variable required"))?;

        let redis_client = redis::Client::open(redis_url)?;

        let window_minutes = env::var("RATE_LIMIT_WINDOW_MINUTES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1);

        Ok(Self {
            redis_client,
            window_seconds: window_minutes * 60,
        })
    }

    /// Get rate limit from environment variables 
    pub fn get_rate_limit_for_role(&self, role: &str, endpoint: &str) -> u32 {
        match (role, endpoint) {
            // Guest users100/menit
            ("guest", _) => env::var(RATE_LIMIT_GUEST)
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(100),

            // Customer users 
            ("customer", "/api/auth/register") => {
                env::var(RATE_LIMIT_SENSITIVE)
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(10)
            },
            ("customer", "/api/auth/login") => {
                env::var(RATE_LIMIT_SENSITIVE)
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(30)
            },
            ("customer", _) => env::var(RATE_LIMIT_CUSTOMER)
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(300),

            // Seller users
            ("seller", _) => env::var(RATE_LIMIT_SELLER)
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(500),

            // Default fallback
            _ => 100,
        }
    }

    /// Check rate limit menggunakan Redis
    pub async fn check_rate_limit(&self, key: &str, role: &str, endpoint: &str) -> bool {
        let max_requests = self.get_rate_limit_for_role(role, endpoint);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let redis_key = format!("rate_limit:{}", key);
        let window_start = (now / self.window_seconds) * self.window_seconds;

        // Try to get Redis connection with proper error handling
        let mut conn = match self.redis_client.get_multiplexed_async_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                tracing::error!("Redis connection failed: {}. Failing open for security.", e);
                return true; 
            }
        };

        // Try to get current rate limit data from Redis
        let current_data: Option<String> = match redis::cmd("GET")
            .arg(&redis_key)
            .query_async(&mut conn)
            .await
        {
            Ok(data) => data,
            Err(e) => {
                tracing::warn!("Redis GET failed: {}. Failing open for security.", e);
                return true;
            }
        };

        if let Some(data_str) = current_data {
            // Try to deserialize existing data
            match serde_json::from_str::<RateLimitData>(&data_str) {
                Ok(data) => {
                    // Check if we're in the same window
                    if data.window_start == window_start {
                        // Same window - check count
                        if data.count >= max_requests {
                            return false; 
                        } else {
                            // Increment count
                            let new_data = RateLimitData {
                                count: data.count + 1,
                                window_start,
                            };

                            if let Err(e) = redis::cmd("SET")
                                .arg(&redis_key)
                                .arg(serde_json::to_string(&new_data).unwrap_or_default())
                                .arg("EX")
                                .arg(self.window_seconds + 10)
                                .query_async::<String>(&mut conn)
                                .await
                            {
                                tracing::warn!("Redis SET failed: {}. Continuing for safety.", e);
                            }

                            return true;
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to deserialize rate limit data: {}. Resetting counter.", e);
                }
            }
        }

        // New window or no existing data 
        let new_data = RateLimitData {
            count: 1,
            window_start,
        };

        if let Err(e) = redis::cmd("SET")
            .arg(&redis_key)
            .arg(serde_json::to_string(&new_data).unwrap_or_default())
            .arg("EX")
            .arg(self.window_seconds + 10)
            .query_async::<()>(&mut conn)
            .await
        {
            tracing::warn!("Redis SET failed: {}. Continuing for safety.", e);
        }

        true
    }
}


/// Extract client IP untuk rate limiting (SECURITY_RULES.md Compliance)
fn extract_client_ip(request: &Request) -> String {
    // Try X-Forwarded-For header (proxy/load balancer)
    if let Some(forwarded) = request.headers().get("x-forwarded-for") {
        if let Ok(forwarded_str) = forwarded.to_str() {
            if let Some(first_ip) = forwarded_str.split(',').next() {
                return first_ip.trim().to_string();
            }
        }
    }

    // Try X-Real-IP header
    if let Some(real_ip) = request.headers().get("x-real-ip") {
        if let Ok(real_ip_str) = real_ip.to_str() {
            return real_ip_str.to_string();
        }
    }

    // Fallback ke connection remote addr
    "unknown".to_string()
}

/// Rate limiting middleware dengan Redis backend 
pub async fn auth_rate_limit_middleware(
    State(state): State<crate::config::AppState>,
    request: Request,
    next: Next,
) -> Result<Response, std::convert::Infallible> {
    let client_ip = extract_client_ip(&request);

    // Extract user role from JWT token untuk granular rate limiting 
    let user_role = if let Some(auth_header) = request.headers().get(header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Bearer ") {
                let token = &auth_str[7..];
                match crate::utils::jwt::validate_token(token, &state.config.jwt_secret, &state.db).await {
                    Ok(claims) => claims.role,
                    Err(_) => "guest".to_string(),
                }
            } else {
                "guest".to_string()
            }
        } else {
            "guest".to_string()
        }
    } else {
        "guest".to_string()
    };

    let endpoint = request.uri().path();
    let rate_limit_key = format!("{}:{}", client_ip, user_role);

    // Check rate limit dengan Redis backend 
    if !state.rate_limiter.check_rate_limit(&rate_limit_key, &user_role, endpoint).await {
        tracing::warn!("Rate limit exceeded for IP: {} with role: {} on endpoint: {}",
            client_ip, user_role, endpoint);

        let mut response = axum::response::Response::new(axum::body::Body::from("Rate limit exceeded"));
        *response.status_mut() = StatusCode::TOO_MANY_REQUESTS;

        let headers = response.headers_mut();
        headers.insert("Retry-After", "60".parse().unwrap());
        headers.insert("X-RateLimit-Limit",
            state.rate_limiter.get_rate_limit_for_role(&user_role, endpoint).to_string().parse().unwrap());
        headers.insert("X-RateLimit-Remaining", "0".parse().unwrap());
        headers.insert(
            "X-RateLimit-Reset",
            (std::time::SystemTime::now() + Duration::from_secs(60))
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
                .to_string()
                .parse()
                .unwrap(),
        );

        return Ok(response);
    }

    Ok(next.run(request).await)
}
