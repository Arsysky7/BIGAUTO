use reqwest::header::HeaderMap;

// Extract Authorization header dari request
pub fn extract_auth_header(headers: &HeaderMap) -> Option<String> {
    headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .map(|s| s.to_string())
}


// Extract bearer token dari Authorization header
pub fn extract_bearer_token(auth_header: &str) -> Option<String> {
    if auth_header.starts_with("Bearer ") {
        Some(auth_header[7..].to_string())
    } else {
        None
    }
}

// Extract client IP dari headers
pub fn extract_client_ip(headers: &HeaderMap) -> Option<String> {
    if let Some(forwarded) = headers.get("x-forwarded-for") {
        if let Ok(forwarded_str) = forwarded.to_str() {
            return forwarded_str.split(',').next().map(|s| s.trim().to_string());
        }
    }

    if let Some(real_ip) = headers.get("x-real-ip") {
        if let Ok(real_ip_str) = real_ip.to_str() {
            return Some(real_ip_str.to_string());
        }
    }

    None
}

// Extract User-Agent header
pub fn extract_user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get("user-agent")
        .and_then(|value| value.to_str().ok())
        .map(|s| s.to_string())
}