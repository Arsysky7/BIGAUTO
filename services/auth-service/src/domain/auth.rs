use crate::config::AppState;
use crate::error::AppError;
use crate::models::{
    email_verification::{EmailVerification, NewEmailVerification},
    login_otp::{LoginOtp, NewLoginOtp},
    session::{NewUserSession, UserSession},
    user::{NewUser, User},
};
// Import utilities directly from submodules
use crate::utils::{email, hash, jwt, otp, validation};
use chrono::{Duration, Utc};
use redis::AsyncCommands;
use uuid::Uuid;
use sha2::{Digest, Sha256};
// Menentukan role user untuk JWT claims berdasarkan status customer/seller
fn determine_user_role(user: &User) -> String {
    user.get_jwt_role()
}


// struktur data untuk registrasi
#[derive(Debug, serde::Deserialize)]
pub struct RegisterInput {
    pub email: String,
    pub password: String,
    pub name: String,
    pub phone: String,
    pub address: Option<String>,
    pub city: Option<String>,
}

// Struktur data untuk response registrasi
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct RegisterResponse {
    #[schema(example = 1)]
    pub user_id: i32,
    #[schema(example = "john@example.com")]
    pub email: String,
    #[schema(example = "John Doe")]
    pub name: String,
    #[schema(example = "Registrasi berhasil. Silakan cek email untuk verifikasi akun.")]
    pub message: String,
}

// Struktur data untuk input login step 1
#[derive(Debug, serde::Deserialize)]
pub struct LoginStep1Input {
    pub email: String,
    pub password: String,
}

// Struktur data untuk input login step 2
#[derive(Debug, serde::Deserialize)]
pub struct LoginStep2Input {
    pub user_id: i32,
    pub otp_code: String,
}

// Struktur data untuk response login lengkap
#[derive(Debug, serde::Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub user: UserData,
    pub expires_in: i64,
}

// Data user untuk response (tanpa sensitive info)
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct UserData {
    #[schema(example = 1)]
    pub id: i32,
    #[schema(example = "john@example.com")]
    pub email: String,
    #[schema(example = "John Doe")]
    pub name: String,
    #[schema(example = "+6281234567890")]
    pub phone: String,
    #[schema(example = false)]
    pub is_seller: bool,
    #[schema(example = true)]
    pub email_verified: bool,
}


impl From<User> for UserData {
    fn  from(user: User) -> Self {
        UserData {
            id: user.id,
            email: user.email,
            name: user.name,
            phone: user.phone,
            is_seller: user.is_seller.unwrap_or(false),
            email_verified: user.email_verified.unwrap_or(false),
        }
    }
}

// Registrasi user baru dan kirim email verifikasi
pub async fn register_user(
    state: &AppState,
    input: RegisterInput,
) -> Result<RegisterResponse, AppError> {
    // Validasi input data
    validation::validate_email(&input.email)
        .map_err(|e| AppError::ValidationError(e))?;
    validation::validate_password(&input.password)
        .map_err(|e| AppError::ValidationError(e))?;
    validation::validate_phone(&input.phone)
        .map_err(|e| AppError::ValidationError(e))?;


    // Cek apakah email sudah terdaftar
    if User::find_by_email(&state.db, &input.email).await?.is_some() {
        return Err(AppError::conflict("Email sudah terdaftar"));
    }

    // Hash password
    let password_hash = hash::hash_password(&input.password)
        .map_err(|e| AppError::InternalError(format!("Gagal hash password: {}", e)))?;

    // Buat user baru di database
    let new_user = NewUser {
        email: input.email.to_lowercase().trim().to_string(),
        password_hash,
        name: input.name.trim().to_string(),
        phone: validation::normalize_phone(&input.phone),
        address: input.address.map(|a| a.trim().to_string()),
        city: input.city.map(|c| c.trim().to_string()),
    };

    let user = User::create(&state.db, new_user).await?;

    // Geneerate Token Verifikasi Email
    let verification_token = Uuid::new_v4().to_string();
    let expires_at = Utc::now() + Duration::hours(24);

    // Simpan token verifikasi di database
    let verification_data = NewEmailVerification {
        user_id: user.id,
        token: verification_token.clone(),
        email: user.email.clone(),
        expires_at,
    };

    EmailVerification::create(&state.db, verification_data).await?;

    // Clone data untuk response sebelum move ke tokio::spawn
    let response = RegisterResponse {
        user_id: user.id,
        email: user.email.clone(),
        name: user.name.clone(),
        message: "Registrasi berhasil. Silakan cek email untuk verifikasi akun.".to_string(),
    };

    // Kirim email verifikasi (async, non-blocking)
    let http_client = state.http_client.clone();
    let api_key = state.config.email_config.resend_api_key.clone();
    let from_email = state.config.email_config.email_from.clone();
    tokio::spawn(async move {
        if let Err(e) = email::send_verification_email(
            &http_client,
            &api_key,
            &from_email,
            &user.email,
            &user.name,
            &verification_token,
        ).await {
            tracing::error!("Gagal mengirim email verifikasi: {}", e);
        }
    });

    Ok(response)
}


// Verifikasi email menggunakan Token dari email
pub async fn verify_email(
    state: &AppState,
    token: &str,
) -> Result<String, AppError> {
    // cari verifikasi berdasarkan token
    let verification = EmailVerification::find_by_token(&state.db, token).await?
        .ok_or_else(|| AppError::NotFoundError("Token verfikasi tidak ditemukan".to_string()))?;
    

    // Gunakan method is_valid() untuk validasi token
    if !verification.is_valid() {
        if verification.is_used.unwrap_or(false) {
            return Err(AppError::validation(
                "Token sudah pernah digunakan"
            ));
        } else {
            return Err(AppError::validation(
                "Token sudah kadaluarsa. Silakan request token baru."
            ));
        }
    }

    // Mark token sebagai used
    EmailVerification::mark_as_used(&state.db, verification.id).await?;

    // Update user email_verified status
    User::verify_email(&state.db, verification.user_id).await?;

    Ok("Email berhasil diverifikasi. Silakan login".to_string())
}

// Kirim ulang email verifikasi jika user belum terverifikasi
pub async fn resend_verification(
    state: &AppState,
    email: &str,
) -> Result<String, AppError> {
    // Cari user berdasarkan email
    let user = User::find_by_email(&state.db, email)
        .await?
        .ok_or_else(|| AppError::NotFoundError("User tidak ditemukan".to_string()))?;

    // Cek apakah email sudah verified
    if user.email_verified.unwrap_or(false) {
        return Err(AppError::ValidationError(
            "Email sudah diverifikasi".to_string(),
        ));
    }

    // Rate limiting: cek apakah user sudah request terlalu sering
    let rate_key = format!("resend_verification:{}", user.id);
    let mut redis = state.redis.clone();
    let count: Option<i32> = redis.get(&rate_key).await?;

    if let Some(c) = count {
        if c >= 3 {
            return Err(AppError::RateLimitError(
                "Terlalu banyak permintaan. Coba lagi dalam 1 jam.".to_string(),
            ));
        }
    }

    // Increment rate limit counter
    let _: () = redis.incr(&rate_key, 1).await?;
    let _: () = redis.expire(&rate_key, 3600).await?;

    // Generate token baru
    let verification_token = Uuid::new_v4().to_string();
    let expires_at = Utc::now() + Duration::hours(24);

    let verification_data = NewEmailVerification {
        user_id: user.id,
        token: verification_token.clone(),
        email: user.email.clone(),
        expires_at,
    };

    EmailVerification::create(&state.db, verification_data).await?;

    // Update sent_count di latest verification
    if let Some(latest) = EmailVerification::find_latest_by_user(&state.db, user.id).await? {
        EmailVerification::increment_sent_count(&state.db, latest.id).await?;
    }

    // Kirim Email
    let http_client = state.http_client.clone();
    let api_key = state.config.email_config.resend_api_key.clone();
    let from_email = state.config.email_config.email_from.clone();
    tokio::spawn(async move {
        if let Err(e) = email::send_verification_email(
            &http_client,
            &api_key,
            &from_email,
            &user.email,
            &user.name,
            &verification_token
        ).await {
            tracing::error!("Gagal mengirim email: {}", e);
        }
    });

    Ok("Email verifikasi telah dikirim ulang. Silakan cek inbox Anda.".to_string())
    
}

// Login ste 1: validasi kredensial dan kriim OTP ke email
pub async fn login_step1_send_otp(
    state: &AppState,
    input: LoginStep1Input,
    ip_address: Option<String>,
    user_agent: Option<String>,
) -> Result<i32, AppError> {
    // cari user berdasarkan email
    let user = User::find_by_email(&state.db, &input.email)
        .await?
        .ok_or_else(|| AppError::AuthenticationError("Email atau password salah".to_string()))?;

    // Validasi password
    let password_valid = hash::verify_password(&input.password, &user.password_hash)
        .map_err(|e| AppError::InternalError(format!("Gagal verifikasi password: {}", e)))?;

    if !password_valid {
        return Err(AppError::AuthenticationError(
            "Email atau password salah".to_string(),
        ));
    }

    // Cek apakah email sudah verified 
    if !user.email_verified.unwrap_or(false) {
        return Err(AppError::AuthenticationError(
            "Email belum diverifikasi. Silakan cek inbox Anda.".to_string(),
        ));
    }
    
    // Cek apakah akun aktif
    if !user.is_active.unwrap_or(true) {
        return Err(AppError::AuthenticationError(
            "Akun Anda telah dinonaktifkan. Hubungi support.".to_string(),
        ));
    }


    // Rate limiting OTP: cek apakah user sedang di blok 
    if let Some(blocked_until) = user.otp_blocked_until {
        if blocked_until > Utc::now() {
            let remaining = (blocked_until - Utc::now()).num_minutes();
            return Err(AppError::RateLimitError(format!(
                "Akun diblokir karena terlalu banyak percobaan. Coba lagi dalam {} menit.",
                remaining
            )));
        }
    }

    // Rate limiting: max 5 OTP request per jam
    let rate_key = format!("otp_request:{}", user.id);
    let mut redis = state.redis.clone();
    let count: Option<i32> = redis.get(&rate_key).await?;

    if let Some(cnt) = count {
        if cnt >= 5 {
            // Block user selama 1 jam (60 menit)
            User::block_otp_requests(&state.db, user.id, 60).await?;
            return Err(AppError::RateLimitError(
                "Terlalu banyak permintaan OTP. Akun diblokir selama 1 jam.".to_string(),
            ));
        }
    }

    // Increment OTP request count
    let _: () = redis.incr(&rate_key, 1).await?;
    let _: () = redis.expire(&rate_key, 3600).await?;

    // Invalidate semua OTP lama untuk user (security best practice)
    LoginOtp::invalidate_old_otps(&state.db, user.id).await?;

    // Generate OTP 6 digit
    let otp_code = otp::generate_otp();
    let otp_hash = hash::hash_password(&otp_code)
        .map_err(|e| AppError::internal(format!("Gagal hash OTP: {}", e)))?;

    // Simpan OTP ke database (valid 5 menit)
    let otp_data = NewLoginOtp {
        user_id: user.id,
        otp_code: otp_code.clone(),
        otp_hash,
        expires_at: Utc::now() + Duration::minutes(5),
        ip_address,
        user_agent,
    };

    LoginOtp::create(&state.db, otp_data).await?;

    // Update last_otp_request_at
    User::increment_otp_request(&state.db, user.id).await?;

    // Kirim OTP via email (async)
    let http_client = state.http_client.clone();
    let api_key = state.config.email_config.resend_api_key.clone();
    let from_email = state.config.email_config.email_from.clone();
    let user_email = user.email.clone();
    let user_name = user.name.clone();
    tokio::spawn(async move {
        if let Err(e) = email::send_otp_email(
            &http_client,
            &api_key,
            &from_email,
            &user_email,
            &user_name,
            &otp_code
        ).await {
            tracing::error!("Gagal mengirim OTP email: {}", e);
        }
    });

    Ok(user.id)
}

// Login step 2: verifikasi OTP dan generate JWT tokens
pub async fn login_step2_verify_otp(
    state: &AppState,
    input: LoginStep2Input,
    ip_address: Option<String>,
    user_agent: Option<String>,
) -> Result<LoginResponse, AppError> {
    // Cari OTP terakhir yang valid untuk user
    let otp_record = LoginOtp::find_latest_valid_by_user(&state.db, input.user_id)
        .await?
        .ok_or_else(|| {
            AppError::AuthenticationError("OTP tidak ditemukan atau sudah kadaluarsa".to_string())
        })?;

    // Gunakan method is_valid() untuk validasi OTP
    if !otp_record.is_valid() {
        if otp_record.is_blocked() {
            return Err(AppError::rate_limit(
                "OTP diblokir karena terlalu banyak percobaan gagal."
            ));
        } else {
            return Err(AppError::authentication(
                "OTP sudah kadaluarsa. Silakan request OTP baru."
            ));
        }
    }

    let attempt_count = otp_record.attempt_count.unwrap_or(0);
    if attempt_count >= 3 {
        
        LoginOtp::block_otp(&state.db, otp_record.id, 15).await?;
        return Err(AppError::RateLimitError(
            "Terlalu banyak percobaan gagal. OTP diblokir selama 15 menit.".to_string(),
        ));
    }

    // Verifikasi OTP code
    let otp_valid = hash::verify_password(&input.otp_code, &otp_record.otp_hash)
        .map_err(|e| AppError::InternalError(format!("Gagal verifikasi OTP: {}", e)))?;

    if !otp_valid {
        // Increment attempt count
        LoginOtp::increment_attempt(&state.db, otp_record.id).await?;

        let remaining = 3 - attempt_count - 1;
        return Err(AppError::AuthenticationError(format!(
            "Kode OTP salah. Sisa percobaan: {}",
            remaining
        )));
    }

    // OTP valid, mark OTP sebagai used
    LoginOtp::mark_as_used(&state.db, otp_record.id).await?;

    // Load user data
    let user = User::find_by_id(&state.db, input.user_id)
        .await?
        .ok_or_else(|| AppError::NotFoundError("User tidak ditemukan".to_string()))?;

    // Determine role based on user data 
    let role = determine_user_role(&user);


    // Generate JWT tokens menggunakan config dari AppState
    let access_token = jwt::generate_access_token(
        user.id,
        &user.email,
        &role,
        &state.config.jwt_secret,
        state.config.jwt_access_expiry
    )?;

    // Extract JTI dari access token untuk tracking
    let claims = jwt::validate_token(&access_token, &state.config.jwt_secret, &state.db)
        .await?;
    let access_jti = claims.jti.clone();
    let refresh_token = jwt::generate_refresh_token(
        user.id,
        &user.email,
        &role,
        &state.config.jwt_secret,
        state.config.jwt_refresh_expiry
    )?;

    // Create user session WITH JTI tracking
    let session_data = NewUserSession {
        user_id: user.id,
        refresh_token: refresh_token.clone(),
        access_token_jti: Some(access_jti.clone()),
        user_agent,
        ip_address,
        device_name: None,
        expires_at: Utc::now() + Duration::days(7),
    };

    let _session = UserSession::create(&state.db, session_data).await?;

    // Update user login statistics
    User::update_login_tracking(&state.db, user.id).await?;

    tracing::info!(
        "User {} logged in successfully with JTI: {}",
        user.id,
        access_jti
    );

    // Clear rate limiting counters
    let mut redis = state.redis.clone();
    let rate_key = format!("otp_request:{}", user.id);
    let _: () = redis.del(&rate_key).await?;

    Ok(LoginResponse {
        access_token,
        refresh_token,
        user: user.into(),
        expires_in: state.config.jwt_access_expiry,
    })

}

// Kirim ulang OTP jika user belum input atau expired
pub async fn resend_otp(
    state: &AppState,
    user_id: i32,
    ip_address: Option<String>,
    user_agent: Option<String>,
) -> Result<String, AppError> {
    // Load user
    let user = User::find_by_id(&state.db, user_id)
        .await?
        .ok_or_else(|| AppError::NotFoundError("User tidak ditemukan".to_string()))?;

    // Rate limiting: cooldown 60 detik antar request
    let cooldown_key = format!("otp_cooldown:{}", user.id);
    let mut redis = state.redis.clone();
    let exists: bool = redis.exists(&cooldown_key).await?;

    if exists {
        return Err(AppError::RateLimitError(
            "Tunggu 60 detik sebelum request OTP lagi.".to_string(),
        ));
    }

    // Set cooldown
    let _: () = redis.set_ex(&cooldown_key, 1, 60).await?;

    // Rate limiting: max 5 request per jam (sama seperti login)
    let rate_key = format!("otp_request:{}", user.id);
    let count: Option<i32> = redis.get(&rate_key).await?;

    if let Some(cnt) = count {
        if cnt >= 5 {
            return Err(AppError::RateLimitError(
                "Terlalu banyak permintaan OTP. Coba lagi dalam 1 jam.".to_string(),
            ));
        }
    }

    // Increment counter
    let _: () = redis.incr(&rate_key, 1).await?;
    let _: () = redis.expire(&rate_key, 3600).await?;

    // Invalidate semua OTP lama untuk user (security best practice)
    LoginOtp::invalidate_old_otps(&state.db, user.id).await?;

    // Generate OTP baru
    let otp_code = otp::generate_otp();
    let otp_hash = hash::hash_password(&otp_code)
        .map_err(|e| AppError::internal(format!("Gagal hash OTP: {}", e)))?;

    let otp_data = NewLoginOtp {
        user_id: user.id,
        otp_code: otp_code.clone(),
        otp_hash,
        expires_at: Utc::now() + Duration::minutes(5),
        ip_address,
        user_agent,
    };

    LoginOtp::create(&state.db, otp_data).await?;

    // Kirim email
    let http_client = state.http_client.clone();
    let api_key = state.config.email_config.resend_api_key.clone();
    let from_email = state.config.email_config.email_from.clone();
    let user_email = user.email.clone();
    let user_name = user.name.clone();
    tokio::spawn(async move {
        if let Err(e) = email::send_otp_email(
            &http_client,
            &api_key,
            &from_email,
            &user_email,
            &user_name,
            &otp_code
        ).await {
            tracing::error!("Gagal mengirim OTP: {}", e);
        }
    });

    Ok("OTP baru telah dikirim ke email Anda.".to_string())
}

// Refresh access token menggunakan refresh token
pub async fn refresh_access_token(
    state: &AppState,
    refresh_token: &str,
) -> Result<String, AppError> {
    // Validasi refresh token
    let claims = jwt::validate_token(refresh_token, &state.config.jwt_secret, &state.db)
        .await?;

    // Cek apakah token type = refresh
    if claims.token_type != "refresh" {
        return Err(AppError::TokenError(
            "Token bukan refresh token".to_string(),
        ));
    }

    // cari session bedasarkan refresh token 
    let session = UserSession::find_by_refresh_token(&state.db, refresh_token)
        .await?
        .ok_or_else(|| AppError::AuthenticationError("Session tidak ditemukan".to_string()))?;

    // Gunakan method is_valid() untuk validasi session
    if !session.is_valid() {
        return Err(AppError::authentication(
            "Session sudah kadaluarsa. Silakan login ulang"
        ));
    }

    // Load user untuk generate token baru
    let user = User::find_by_id(&state.db, session.user_id)
        .await?
        .ok_or_else(|| AppError::NotFoundError("User tidak ditemukan".to_string()))?;

    // Determine role based on user data
    let role = determine_user_role(&user);

    // Generate access token
    let new_access_token = jwt::generate_access_token(
        user.id,
        &user.email,
        &role,
        &state.config.jwt_secret,
        state.config.jwt_access_expiry
    )?;

    // Extract JTI dari new access token
    let new_claims = jwt::validate_token(&new_access_token, &state.config.jwt_secret, &state.db)
        .await?;
    let new_jti = new_claims.jti.clone();

    // Update session dengan JTI baru untuk tracking
    UserSession::update_access_token_jti(&state.db, session.id, &new_jti).await?;

    // Update last_activity di session
    UserSession::update_activity(&state.db, session.id).await?;

    tracing::info!("Access token refreshed for user {} with new JTI: {}", user.id, new_jti);

    Ok(new_access_token)
}



/// Logout user dengan keamanan enterprise: blacklist semua JWT tokens
pub async fn logout(state: &AppState, refresh_token: &str) -> Result<String, AppError> {
    let token_hash = hash_token_for_logging(refresh_token);
    tracing::info!("Processing logout request - token: {}...", token_hash);

    // Extract data dari refresh token untuk tracking
    let token_claims = validate_refresh_token(refresh_token, &state.config.jwt_secret, &state.db).await?;
    let user_id = token_claims.sub;
    let refresh_jti = token_claims.jti;

    // Execute security-critical operations secara atomik
    execute_logout_security_procedures(&state, user_id, &refresh_jti).await?;

    tracing::info!("Logout completed successfully - user_id: {}", user_id);
    Ok("Logout berhasil. Semua token telah diblacklist.".to_string())
}

/// Validasi refresh token sebelum proses logout
async fn validate_refresh_token(token: &str, jwt_secret: &str, db: &sqlx::PgPool) -> Result<crate::utils::jwt::TokenClaims, AppError> {
    let claims = crate::utils::jwt::validate_token(token, jwt_secret, db)
        .await
        .map_err(|e| AppError::AuthenticationError(format!("Token tidak valid: {}", e)))?;

    if claims.token_type != "refresh" {
        return Err(AppError::TokenError("Token type tidak sesuai untuk logout".to_string()));
    }

    Ok(claims)
}

/// Hash token untuk logging aman 
fn hash_token_for_logging(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let hash = hasher.finalize();
    format!("{:x}", hash)[..16].to_string()
}

/// Eksekusi prosedur keamanan logout secara atomik
async fn execute_logout_security_procedures(
    state: &AppState,
    user_id: i32,
    refresh_jti: &str,
) -> Result<(), AppError> {
    let mut tx = state.db.begin().await
        .map_err(|e| AppError::InternalError(format!("Gagal memulai transaksi database: {}", e)))?;

    // Blacklist refresh token dengan alasan dan expiry
    blacklist_jwt_token(&mut tx, refresh_jti, "refresh", user_id, "user_logout").await?;

    // Cari dan blacklist semua access tokens terkait
    blacklist_related_access_tokens(&mut tx, user_id, refresh_jti).await?;

    // Commit semua perubahan keamanan
    tx.commit().await
        .map_err(|e| AppError::InternalError(format!("Gagal commit transaksi logout: {}", e)))?;

    // Invalidate user sessions di background untuk performance
    let db_clone = state.db.clone();
    tokio::spawn(async move {
        if let Err(e) = invalidate_user_sessions(&db_clone, user_id).await {
            tracing::error!("Failed to invalidate user sessions: {}", e);
        }
    });

    Ok(())
}

/// Blacklist JWT token menggunakan secure function 
async fn blacklist_jwt_token(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    jti: &str,
    token_type: &str,
    user_id: i32,
    reason: &str,
) -> Result<(), AppError> {
    // Gunakan secure function untuk blacklist token
    sqlx::query_scalar!("SELECT blacklist_token($1, $2, $3)",
        jti,
        token_type,
        reason
    )
    .fetch_one(tx.as_mut())
    .await
    .map_err(|e| AppError::InternalError(format!("Gagal blacklist token via secure function: {}", e)))?;

    tracing::info!("Successfully blacklisted token: jti={}, type={}, user_id={}", jti, token_type, user_id);
    Ok(())
}

/// Cari dan blacklist semua access tokens terkait dengan session
async fn blacklist_related_access_tokens(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: i32,
    refresh_jti: &str,
) -> Result<(), AppError> {
    // Cari session terkait dengan refresh token menggunakan query
    let session_data: Option<i32> = sqlx::query_scalar!(
        "SELECT id FROM user_sessions WHERE refresh_token = $1 AND expires_at > NOW() AND is_active = true",
        refresh_jti
    )
    .fetch_optional(tx.as_mut())
    .await?;

    if let Some(session_id) = session_data {
        // Ambil access token JTI dari session
        let access_jti = sqlx::query_scalar!(
            "SELECT access_token_jti FROM user_sessions WHERE id = $1",
            session_id
        )
        .fetch_optional(tx.as_mut())
        .await?
        .flatten(); 

        if let Some(jti) = access_jti {
            blacklist_jwt_token(&mut *tx, jti.as_str(), "access", user_id, "user_logout").await?;
        }

        // Update session menjadi tidak aktif
        sqlx::query!(
            "UPDATE user_sessions SET is_active = false, updated_at = NOW() WHERE id = $1",
            session_id
        )
        .execute(tx.as_mut())
        .await
        .map_err(|e| AppError::InternalError(format!("Gagal update session logout: {}", e)))?;
    } else {
        tracing::debug!("No active session found for refresh token during logout");
    }

    Ok(())
}

/// Invalidate semua session user untuk multi-device logout
async fn invalidate_user_sessions(db: &sqlx::PgPool, user_id: i32) -> Result<(), AppError> {
    // Update semua session user menjadi tidak aktif
    sqlx::query!(
        "UPDATE user_sessions SET is_active = false, updated_at = NOW() WHERE user_id = $1 AND is_active = true",
        user_id
    )
    .execute(db)
    .await
    .map_err(|e| AppError::InternalError(format!("Gagal invalidate user sessions: {}", e)))?;

    tracing::info!("Invalidated all sessions for user_id: {}", user_id);
    Ok(())
}