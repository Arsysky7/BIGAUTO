use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use time::Duration;
use utoipa::ToSchema;

use crate::{
    config::AppState,
    domain::auth::{
        self as auth_domain, LoginStep1Input, LoginStep2Input, RegisterInput,
        RegisterResponse, UserData,
    },
    error::{AppError, AppResult},
};

// ===== REQUEST DTOs =====

/// Request body untuk register endpoint
#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterRequestBody {
    #[schema(example = "john@example.com")]
    pub email: String,
    #[schema(example = "SecureP@ss123")]
    pub password: String,
    #[schema(example = "John Doe")]
    pub name: String,
    #[schema(example = "+6281234567890")]
    pub phone: String,
    #[schema(example = "Jl. Sudirman No. 123")]
    pub address: Option<String>,
    #[schema(example = "Jakarta")]
    pub city: Option<String>,
}

/// Query parameter untuk verify email
#[derive(Debug, Deserialize, ToSchema)]
pub struct VerifyEmailQuery {
    #[schema(example = "abc123def456")]
    pub token: String,
}

/// Request body untuk resend verification
#[derive(Debug, Deserialize, ToSchema)]
pub struct ResendVerificationRequest {
    #[schema(example = "john@example.com")]
    pub email: String,
}

/// Request body untuk login step 1
#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequestBody {
    #[schema(example = "john@example.com")]
    pub email: String,
    #[schema(example = "SecureP@ss123")]
    pub password: String,
}

/// Request body untuk verify OTP (login step 2)
#[derive(Debug, Deserialize, ToSchema)]
pub struct VerifyOtpRequestBody {
    #[schema(example = 1)]
    pub user_id: i32,
    #[schema(example = "123456")]
    pub otp_code: String,
}

/// Request body untuk resend OTP
#[derive(Debug, Deserialize, ToSchema)]
pub struct ResendOtpRequest {
    #[schema(example = 1)]
    pub user_id: i32,
}

// ===== RESPONSE DTOs =====

/// Response dengan message sukses
#[derive(Debug, Serialize, ToSchema)]
pub struct MessageResponse {
    #[schema(example = "Operasi berhasil")]
    pub message: String,
}

/// Response login step 1
#[derive(Debug, Serialize, ToSchema)]
pub struct LoginStep1Response {
    #[schema(example = "OTP telah dikirim ke email Anda")]
    pub message: String,
    #[schema(example = 1)]
    pub user_id: i32,
}

/// Response login step 2 (sukses login)
#[derive(Debug, Serialize, ToSchema)]
pub struct LoginStep2Response {
    #[schema(example = "eyJhbGciOiJIUzI1NiIs...")]
    pub access_token: String,
    pub user: UserData,
    #[schema(example = "Login berhasil")]
    pub message: String,
}

/// Response refresh token
#[derive(Debug, Serialize, ToSchema)]
pub struct RefreshTokenResponse {
    #[schema(example = "eyJhbGciOiJIUzI1NiIs...")]
    pub access_token: String,
}

// ===== HELPER FUNCTIONS =====

// Ekstrak IP address dari request headers untuk security tracking
fn extract_ip_address(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
        })
}

// Ekstrak user agent dari request headers untuk device tracking
fn extract_user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

// ===== HANDLER FUNCTIONS =====

/// Register user baru dengan email verification
#[utoipa::path(
    post,
    path = "/api/auth/register",
    request_body = RegisterRequestBody,
    responses(
        (status = 201, description = "User berhasil didaftarkan", body = RegisterResponse),
        (status = 400, description = "Validation error"),
        (status = 409, description = "Email sudah terdaftar")
    ),
    tag = "Authentication"
)]
pub async fn register_handler(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequestBody>,
) -> AppResult<impl IntoResponse> {
    // Convert request body ke domain input
    let input = RegisterInput {
        email: req.email,
        password: req.password,
        name: req.name,
        phone: req.phone,
        address: req.address,
        city: req.city,
    };

    // Call domain layer untuk business logic
    let response = auth_domain::register_user(&state, input).await?;

    // Return response dengan status CREATED
    Ok((
        StatusCode::CREATED,
        Json(response),
    ))
}

/// Verifikasi email menggunakan token dari link email
#[utoipa::path(
    get,
    path = "/api/auth/verify-email",
    params(
        ("token" = String, Query, description = "Token verifikasi dari email")
    ),
    responses(
        (status = 200, description = "Email berhasil diverifikasi", body = MessageResponse),
        (status = 400, description = "Token invalid atau expired")
    ),
    tag = "Authentication"
)]
pub async fn verify_email_handler(
    State(state): State<AppState>,
    Query(query): Query<VerifyEmailQuery>,
) -> AppResult<impl IntoResponse> {
    // Verify token melalui domain layer
    auth_domain::verify_email(&state, &query.token).await?;

    let response = MessageResponse {
        message: "Email berhasil diverifikasi. Silakan login.".to_string(),
    };

    Ok(Json(response))
}

/// Kirim ulang email verifikasi jika expired atau tidak diterima
#[utoipa::path(
    post,
    path = "/api/auth/resend-verification",
    request_body = ResendVerificationRequest,
    responses(
        (status = 200, description = "Email verifikasi berhasil dikirim ulang", body = MessageResponse),
        (status = 400, description = "Email tidak ditemukan atau sudah terverifikasi"),
        (status = 429, description = "Terlalu banyak permintaan, tunggu beberapa saat")
    ),
    tag = "Authentication"
)]
pub async fn resend_verification_handler(
    State(state): State<AppState>,
    Json(req): Json<ResendVerificationRequest>,
) -> AppResult<impl IntoResponse> {
    // Resend verification melalui domain layer (includes rate limiting)
    auth_domain::resend_verification(&state, &req.email).await?;

    let response = MessageResponse {
        message: "Email verifikasi telah dikirim ulang. Silakan cek inbox Anda.".to_string(),
    };

    Ok(Json(response))
}

/// Login step 1: Validasi email+password dan kirim OTP ke email
#[utoipa::path(
    post,
    path = "/api/auth/login",
    request_body = LoginRequestBody,
    responses(
        (status = 200, description = "OTP berhasil dikirim ke email", body = LoginStep1Response),
        (status = 400, description = "Email atau password salah"),
        (status = 403, description = "Email belum diverifikasi")
    ),
    tag = "Authentication"
)]
pub async fn login_step1_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<LoginRequestBody>,
) -> AppResult<impl IntoResponse> {
    // Ekstrak IP dan user agent untuk security tracking
    let ip_address = extract_ip_address(&headers);
    let user_agent = extract_user_agent(&headers);

    // Convert request ke domain input
    let input = LoginStep1Input {
        email: req.email,
        password: req.password,
    };

    // Call domain layer untuk validasi dan kirim OTP
    let user_id = auth_domain::login_step1_send_otp(&state, input, ip_address, user_agent).await?;

    let response = LoginStep1Response {
        message: "OTP telah dikirim ke email Anda. Kode berlaku 5 menit.".to_string(),
        user_id,
    };

    Ok(Json(response))
}

/// Login step 2: Verifikasi OTP dan generate JWT tokens
#[utoipa::path(
    post,
    path = "/api/auth/verify-otp",
    request_body = VerifyOtpRequestBody,
    responses(
        (status = 200, description = "Login berhasil, tokens generated", body = LoginStep2Response),
        (status = 400, description = "OTP invalid atau expired"),
        (status = 429, description = "Terlalu banyak percobaan, akun diblokir")
    ),
    tag = "Authentication"
)]
pub async fn login_step2_handler(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(req): Json<VerifyOtpRequestBody>,
) -> AppResult<impl IntoResponse> {
    // Ekstrak IP dan user agent untuk session tracking
    let ip_address = extract_ip_address(&headers);
    let user_agent = extract_user_agent(&headers);

    // Convert request ke domain input
    let input = LoginStep2Input {
        user_id: req.user_id,
        otp_code: req.otp_code,
    };

    // Call domain layer untuk verify OTP dan generate tokens
    let login_response = auth_domain::login_step2_verify_otp(&state, input, ip_address, user_agent).await?;

    // Set refresh token di HttpOnly cookie (tidak bisa diakses JavaScript - SECURE!)
    let cookie = axum_extra::extract::cookie::Cookie::build(("refresh_token", login_response.refresh_token.clone()))
        .path("/")
        .max_age(Duration::days(7))
        .http_only(true) // Prevent XSS attacks
        .secure(true) // HTTPS only di production
        .same_site(axum_extra::extract::cookie::SameSite::Strict) // Prevent CSRF attacks
        .build();

    let jar = jar.add(cookie);

    // Return access token dan user data (refresh token sudah di cookie)
    let response = LoginStep2Response {
        access_token: login_response.access_token,
        user: login_response.user,
        message: "Login berhasil.".to_string(),
    };

    Ok((jar, Json(response)))
}

/// Kirim ulang OTP jika expired atau salah input
#[utoipa::path(
    post,
    path = "/api/auth/resend-otp",
    request_body = ResendOtpRequest,
    responses(
        (status = 200, description = "OTP baru berhasil dikirim ke email", body = MessageResponse),
        (status = 400, description = "User ID tidak valid atau tidak dalam proses login"),
        (status = 429, description = "Cooldown aktif, tunggu 60 detik sebelum request ulang")
    ),
    tag = "Authentication"
)]
pub async fn resend_otp_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ResendOtpRequest>,
) -> AppResult<impl IntoResponse> {
    // Ekstrak IP dan user agent untuk rate limiting
    let ip_address = extract_ip_address(&headers);
    let user_agent = extract_user_agent(&headers);

    // Resend OTP melalui domain layer (includes cooldown 60 detik)
    auth_domain::resend_otp(&state, req.user_id, ip_address, user_agent).await?;

    let response = MessageResponse {
        message: "OTP baru telah dikirim ke email Anda.".to_string(),
    };

    Ok(Json(response))
}

/// Refresh access token menggunakan refresh token dari cookie
#[utoipa::path(
    post,
    path = "/api/auth/refresh",
    responses(
        (status = 200, description = "Access token berhasil di-refresh", body = RefreshTokenResponse),
        (status = 401, description = "Refresh token tidak valid atau expired"),
        (status = 400, description = "Refresh token tidak ditemukan di cookie")
    ),
    tag = "Authentication"
)]
pub async fn refresh_token_handler(
    State(state): State<AppState>,
    jar: CookieJar,
) -> AppResult<impl IntoResponse> {
    // Ambil refresh token dari HttpOnly cookie
    let refresh_token = jar
        .get("refresh_token")
        .ok_or_else(|| AppError::authentication("Refresh token tidak ditemukan. Silakan login kembali."))?
        .value()
        .to_string();

    // Generate new access token melalui domain layer
    let access_token = auth_domain::refresh_access_token(&state, &refresh_token).await?;

    let response = RefreshTokenResponse { access_token };

    Ok(Json(response))
}

/// Logout dan invalidate session di database + hapus cookie
#[utoipa::path(
    post,
    path = "/api/auth/logout",
    responses(
        (status = 200, description = "Logout berhasil, session dihapus", body = MessageResponse),
        (status = 400, description = "Tidak ada session aktif")
    ),
    tag = "Authentication"
)]
pub async fn logout_handler(
    State(state): State<AppState>,
    jar: CookieJar,
) -> AppResult<impl IntoResponse> {
    // Ambil refresh token dari cookie jika ada
    if let Some(refresh_token_cookie) = jar.get("refresh_token") {
        let refresh_token = refresh_token_cookie.value().to_string();

        // Invalidate session di database melalui domain layer
        let _ = auth_domain::logout(&state, &refresh_token).await;
    }

    // Hapus refresh token cookie dengan max_age 0
    let cookie = axum_extra::extract::cookie::Cookie::build(("refresh_token", ""))
        .path("/")
        .max_age(Duration::seconds(0))
        .http_only(true)
        .secure(true)
        .same_site(axum_extra::extract::cookie::SameSite::Strict)
        .build();

    let jar = jar.add(cookie);

    let response = MessageResponse {
        message: "Logout berhasil.".to_string(),
    };

    Ok((jar, Json(response)))
}
