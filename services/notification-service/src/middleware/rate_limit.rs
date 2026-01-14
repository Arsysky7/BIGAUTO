// Redis-based Rate Limiting Middleware for Notification Service

use axum::{
    extract::{Request, State},
    http::StatusCode,
    response::{Response, IntoResponse},
    middleware::Next,
};
use redis::{Client, AsyncCommands};
use std::env;
use thiserror::Error;

/// Rate limit configuration dari environment
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub guest_requests_per_hour: u32,
    pub customer_requests_per_hour: u32,
    pub seller_requests_per_hour: u32,
    pub sensitive_requests_per_hour: u32,
    pub window_seconds: u64,
}

impl RateLimitConfig {
    pub fn from_env() -> Self {
        Self {
            guest_requests_per_hour: env::var("RATE_LIMIT_GUEST_REQUESTS")
                .unwrap_or_else(|_| "100".to_string())
                .parse()
                .unwrap_or(100),
            customer_requests_per_hour: env::var("RATE_LIMIT_CUSTOMER_REQUESTS")
                .unwrap_or_else(|_| "300".to_string())
                .parse()
                .unwrap_or(300),
            seller_requests_per_hour: env::var("RATE_LIMIT_SELLER_REQUESTS")
                .unwrap_or_else(|_| "500".to_string())
                .parse()
                .unwrap_or(500),
            sensitive_requests_per_hour: env::var("RATE_LIMIT_SENSITIVE_ENDPOINTS")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .unwrap_or(30),
            window_seconds: 3600,
        }
    }
}

/// Rate limiter menggunakan Redis
#[derive(Clone)]
pub struct RateLimiter {
    redis_client: Client,
    config: RateLimitConfig,
}

impl RateLimiter {
    pub fn new(redis_url: &str) -> Result<Self, RateLimitError> {
        let redis_client = Client::open(redis_url.to_string())
            .map_err(RateLimitError::RedisConnection)?;

        Ok(Self {
            redis_client,
            config: RateLimitConfig::from_env(),
        })
    }

    /// Check rate limit untuk identifier dan role
    pub async fn check_rate_limit(
        &self,
        identifier: &str,
        role: &str,
        endpoint: &str,
    ) -> Result<RateLimitResult, RateLimitError> {
        let mut conn = self.redis_client.get_multiplexed_async_connection()
            .await
            .map_err(RateLimitError::RedisConnection)?;

        let window_key = format!("rate_limit:{}:{}:{}", identifier, role, endpoint);
        let current_time = chrono::Utc::now().timestamp() as u64;
        let window_start = current_time - self.config.window_seconds + 1;

        // Clean old entries dan add new one
        let _: () = conn
            .zrembyscore(&window_key, "-inf", &(window_start - 1))
            .await
            .map_err(RateLimitError::RedisOperation)?;

        let current_count: usize = conn
            .zcard(&window_key)
            .await
            .map_err(RateLimitError::RedisOperation)?;

        let max_requests = self.get_max_requests(role, endpoint);

        // Add current request
        let _: () = conn
            .zadd(&window_key, current_time, current_time)
            .await
            .map_err(RateLimitError::RedisOperation)?;

        // Set expiration untuk cleanup
        let _: () = conn
            .expire(&window_key, self.config.window_seconds as i64)
            .await
            .map_err(RateLimitError::RedisOperation)?;

        let is_allowed = current_count < max_requests as usize;
        let remaining = if is_allowed {
            max_requests.saturating_sub(current_count as u32 + 1)
        } else {
            0
        };

        Ok(RateLimitResult {
            allowed: is_allowed,
            current_count: current_count as u32 + 1,
            max_requests,
            remaining,
            reset_time: (window_start + self.config.window_seconds) as u64,
        })
    }

    fn get_max_requests(&self, role: &str, endpoint: &str) -> u32 {
        // Check jika sensitive endpoint (write operations)
        let is_sensitive = endpoint.contains("/notifications") &&
            (endpoint.contains("POST") || endpoint.contains("PUT") || endpoint.contains("DELETE"));

        if is_sensitive {
            self.config.sensitive_requests_per_hour
        } else {
            match role {
                "guest" => self.config.guest_requests_per_hour,
                "customer" => self.config.customer_requests_per_hour,
                "seller" => self.config.seller_requests_per_hour,
                _ => self.config.guest_requests_per_hour,
            }
        }
    }
}

/// Rate limit result
#[derive(Debug, Clone)]
pub struct RateLimitResult {
    pub allowed: bool,
    pub current_count: u32,
    pub max_requests: u32,
    pub remaining: u32,
    pub reset_time: u64,
}

/// Rate limiting error types
#[derive(Error, Debug)]
pub enum RateLimitError {
    #[error("Redis connection error: {0}")]
    RedisConnection(#[from] redis::RedisError),

    #[error("Redis operation error: {0}")]
    RedisOperation(redis::RedisError),

    #[error("Rate limit configuration error")]
    Configuration,
}

/// Axum middleware untuk rate limiting
pub async fn rate_limit_middleware(
    State(rate_limiter): State<RateLimiter>,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    // Extract client identifier (IP or user ID)
    let identifier = extract_identifier(&request);

    // Extract user role dari JWT claims 
    let role = request.extensions()
        .get::<crate::middleware::auth::AuthUser>()
        .map(|auth| auth.role.as_str())
        .unwrap_or("guest");

    // Extract endpoint path
    let endpoint = request.uri().path();

    // Check rate limit
    match rate_limiter.check_rate_limit(&identifier, role, endpoint).await {
        Ok(result) if result.allowed => {
            // Add rate limit headers
            let mut response = next.run(request).await;
            let headers = response.headers_mut();

            headers.insert("X-RateLimit-Limit",
                result.max_requests.to_string().parse().unwrap());
            headers.insert("X-RateLimit-Remaining",
                result.remaining.to_string().parse().unwrap());
            headers.insert("X-RateLimit-Reset",
                result.reset_time.to_string().parse().unwrap());

            Ok(response)
        }
        Ok(_) => {
            // Rate limit exceeded
            let error_response = axum::Json(serde_json::json!({
                "error": "rate_limit_exceeded",
                "message": "Too many requests. Please try again later."
            }));

            Ok((StatusCode::TOO_MANY_REQUESTS, error_response).into_response())
        }
        Err(e) => {
            tracing::error!("Rate limiting error: {}", e);
            Ok(next.run(request).await)
        }
    }
}

/// Extract client identifier untuk rate limiting
fn extract_identifier(request: &Request) -> String {
    // Try to get user ID dari authenticated user first
    if let Some(auth_user) = request.extensions().get::<crate::middleware::auth::AuthUser>() {
        format!("user:{}", auth_user.user_id)
    } else {
        // Fallback ke IP address
        request.headers()
            .get("x-forwarded-for")
            .or_else(|| request.headers().get("x-real-ip"))
            .and_then(|h| h.to_str().ok())
            .map(|s| s.split(',').next().unwrap_or("").trim().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }
}