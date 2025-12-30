// Redis-based Rate Limiting untuk Chat Service

use axum::{
    extract::{Request, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    middleware::Next,
};
use redis::{Client, AsyncCommands};
use std::env;
use std::sync::Arc;
use thiserror::Error;

// Rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub guest_per_minute: u32,
    pub customer_per_minute: u32,
    pub seller_per_minute: u32,
    pub chat_ops_per_minute: u32,
    pub window_seconds: u64,
}

impl RateLimitConfig {
    pub fn from_env() -> Self {
        let window_minutes = env::var("RATE_LIMIT_WINDOW_MINUTES")
            .unwrap_or_else(|_| "1".to_string())
            .parse()
            .unwrap_or(1);

        Self {
            guest_per_minute: env::var("RATE_LIMIT_GUEST_REQUESTS")
                .unwrap_or_else(|_| "100".to_string())
                .parse()
                .unwrap_or(100),
            customer_per_minute: env::var("RATE_LIMIT_CUSTOMER_REQUESTS")
                .unwrap_or_else(|_| "300".to_string())
                .parse()
                .unwrap_or(300),
            seller_per_minute: env::var("RATE_LIMIT_SELLER_REQUESTS")
                .unwrap_or_else(|_| "500".to_string())
                .parse()
                .unwrap_or(500),
            chat_ops_per_minute: env::var("RATE_LIMIT_SENSITIVE_ENDPOINTS")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .unwrap_or(60),
            window_seconds: window_minutes * 60,
        }
    }
}

// Redis-based rate limiter implementation
#[derive(Clone)]
pub struct RateLimiter {
    redis_client: Client,
    config: RateLimitConfig,
}

impl RateLimiter {
    // Initialize rate limiter dengan Redis connection
    pub fn new(redis_url: &str) -> Result<Self, RateLimitError> {
        let redis_client = Client::open(redis_url.to_string())
            .map_err(RateLimitError::RedisConnection)?;

        tracing::info!("🔗 Chat Service rate limiter connected to Redis");

        Ok(Self {
            redis_client,
            config: RateLimitConfig::from_env(),
        })
    }

    // Check rate limit untuk identifier (user_id atau IP)
    pub async fn check_rate_limit(
        &self,
        identifier: &str,
        role: &str,
        endpoint: &str,
    ) -> Result<RateLimitResult, RateLimitError> {
        let mut conn = self.redis_client
            .get_multiplexed_async_connection()
            .await
            .map_err(RateLimitError::RedisConnection)?;

        // Determine max requests berdasarkan role dan endpoint
        let max_requests = self.get_max_requests(role, endpoint);
        let window_key = format!("chat_rate_limit:{}:{}:{}", identifier, role, endpoint);

        // Gunakan Redis sorted set untuk sliding window
        let current_time = chrono::Utc::now().timestamp() as u64;
        let window_start = current_time.saturating_sub(self.config.window_seconds) + 1;

        // Clean old entries
        let _: () = conn
            .zrembyscore(&window_key, "-inf", &(window_start - 1))
            .await
            .map_err(RateLimitError::RedisOperation)?;

        // Get current count
        let current_count: usize = conn
            .zcard(&window_key)
            .await
            .map_err(RateLimitError::RedisOperation)?;

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

        let allowed = current_count < max_requests as usize;
        let remaining = max_requests.saturating_sub(current_count as u32 + 1);

        Ok(RateLimitResult {
            allowed,
            current_count: current_count as u32 + 1,
            max_requests,
            remaining,
            reset_time: window_start + self.config.window_seconds,
        })
    }

    // Determine max requests berdasarkan role dan endpoint type
    fn get_max_requests(&self, role: &str, endpoint: &str) -> u32 {
        // Chat operations (write) - stricter limit
        let is_chat_write = endpoint.contains("/conversations") ||
            endpoint.contains("/messages") ||
            endpoint.contains("/typing") ||
            endpoint.contains("/upload");

        let is_write_operation = endpoint.contains("POST") ||
            endpoint.contains("PUT") ||
            endpoint.contains("DELETE");

        if is_chat_write && is_write_operation {
            self.config.chat_ops_per_minute
        } else {
            match role {
                "guest" => self.config.guest_per_minute,
                "customer" => self.config.customer_per_minute,
                "seller" => self.config.seller_per_minute,
                _ => self.config.guest_per_minute,
            }
        }
    }
}

// Rate limit check result
#[derive(Debug, Clone)]
pub struct RateLimitResult {
    pub allowed: bool,
    pub current_count: u32,
    pub max_requests: u32,
    pub remaining: u32,
    pub reset_time: u64,
}

// Error types untuk rate limiting
#[derive(Error, Debug)]
pub enum RateLimitError {
    #[error("Redis connection error: {0}")]
    RedisConnection(#[from] redis::RedisError),

    #[error("Redis operation error: {0}")]
    RedisOperation(redis::RedisError),
}

// Axum middleware untuk rate limiting
pub async fn rate_limit_middleware(
    State(rate_limiter): State<Arc<RateLimiter>>,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    // Skip rate limiting untuk health check dan documentation
    let path = request.uri().path();
    if path == "/health" || path.starts_with("/docs") || path.starts_with("/api-docs") || path.starts_with("/redoc") {
        return Ok(next.run(request).await);
    }

    // Skip rate limiting untuk WebSocket (di-handle terpisah)
    if path.contains("/ws/") {
        return Ok(next.run(request).await);
    }

    // Extract identifier dari authenticated user atau IP
    let identifier = extract_identifier(&request);

    // Extract role dari JWT claims
    let role = request
        .extensions()
        .get::<crate::middleware::auth::AuthUser>()
        .map(|auth| auth.role.as_str())
        .unwrap_or("guest");

    // Extract endpoint path
    let endpoint = request.uri().path();

    // Check rate limit
    match rate_limiter.check_rate_limit(&identifier, role, endpoint).await {
        Ok(result) if result.allowed => {
            // Add rate limit headers ke response
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
            tracing::warn!("Rate limit exceeded for {} on {}", identifier, endpoint);

            let error_response = axum::Json(serde_json::json!({
                "error": "rate_limit_exceeded",
                "message": "Terlalu banyak permintaan. Silakan coba lagi dalam beberapa saat.",
                "retry_after": 60
            }));

            Ok((StatusCode::TOO_MANY_REQUESTS, error_response).into_response())
        }
        Err(e) => {
            // Log error tapi allow request
            tracing::error!("Rate limiting error: {}. Allowing request.", e);
            Ok(next.run(request).await)
        }
    }
}

// Extract identifier (user ID atau IP address)
fn extract_identifier(request: &Request) -> String {
    // Prioritaskan user ID dari authenticated user
    if let Some(auth_user) = request.extensions().get::<crate::middleware::auth::AuthUser>() {
        format!("user:{}", auth_user.user_id)
    } else {
        // Fallback ke IP address
        request
            .headers()
            .get("x-forwarded-for")
            .or_else(|| request.headers().get("x-real-ip"))
            .or_else(|| request.headers().get("cf-connecting-ip"))
            .and_then(|h| h.to_str().ok())
            .map(|s| s.split(',').next().unwrap_or("").trim().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }
}