use serde_json::json;
use std::env;

#[derive(Debug, Clone)]
pub struct EmailConfig {
    pub resend_api_key: String,
    pub email_from: String,
}

impl EmailConfig {
    // Load konfigurasi email dari environment variables
    pub fn from_env() -> Result<Self, crate::error::AppError> {
        Ok(EmailConfig {
            resend_api_key: env::var("RESEND_API_KEY").map_err(|_| crate::error::AppError::email("RESEND_API_KEY tidak ditemukan"))?,
            email_from: env::var("RESEND_FROM_EMAIL").unwrap_or_else(|_| "onboarding@resend.dev".to_string()),
        })
    }
}

// Kirim email verifikasi dengan link aktivasi akun menggunakan Resend API
pub async fn send_verification_email(
    http_client: &reqwest::Client,
    api_key: &str,
    from_email: &str,
    to_email: &str,
    to_name: &str,
    verification_token: &str,
) -> Result<(), crate::error::AppError> {
    let verification_link = format!(
        "{}/verify-email?token={}",
        env::var("FRONTEND_URL").expect("FRONTEND_URL environment variable harus diset"),
        verification_token
    );

    let html_body = format!(
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <meta charset="UTF-8">
            <style>
                body {{ font-family: Arial, sans-serif; line-height: 1.6; color: #333; }}
                .container {{ max-width: 600px; margin: 0 auto; padding: 20px; }}
                .header {{ background: #4F46E5; color: white; padding: 20px; text-align: center; }}
                .content {{ background: #f9fafb; padding: 30px; }}
                .button {{ display: inline-block; padding: 12px 30px; background: #4F46E5; color: white; text-decoration: none; border-radius: 5px; }}
                .footer {{ text-align: center; padding: 20px; color: #666; font-size: 12px; }}
            </style>
        </head>
        <body>
            <div class="container">
                <div class="header">
                    <h1>Verifikasi Email Anda</h1>
                </div>
                <div class="content">
                    <p>Halo <strong>{}</strong>,</p>
                    <p>Terima kasih telah mendaftar di Big Auto!</p>
                    <p>Untuk mengaktifkan akun Anda, silakan klik tombol di bawah ini:</p>
                    <p style="text-align: center; margin: 30px 0;">
                        <a href="{}" class="button">Verifikasi Email</a>
                    </p>
                    <p>Atau copy link berikut ke browser Anda:</p>
                    <p style="word-break: break-all; color: #4F46E5;">{}</p>
                    <p><strong>Link ini akan kadaluarsa dalam 24 jam.</strong></p>
                </div>
                <div class="footer">
                    <p>Jika Anda tidak mendaftar di Big Auto, abaikan email ini.</p>
                    <p>&copy; 2025 Big Auto. All rights reserved.</p>
                </div>
            </div>
        </body>
        </html>
        "#,
        to_name, verification_link, verification_link
    );

    send_email_via_resend(
        http_client,
        api_key,
        from_email,
        to_email,
        "Verifikasi Email Anda - Big Auto",
        &html_body
    ).await
}

// Kirim OTP untuk login melalui email menggunakan Resend API
pub async fn send_otp_email(
    http_client: &reqwest::Client,
    api_key: &str,
    from_email: &str,
    to_email: &str,
    to_name: &str,
    otp: &str,
) -> Result<(), crate::error::AppError> {
    let html_body = format!(
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <meta charset="UTF-8">
            <style>
                body {{ font-family: Arial, sans-serif; line-height: 1.6; color: #333; }}
                .container {{ max-width: 600px; margin: 0 auto; padding: 20px; }}
                .header {{ background: #4F46E5; color: white; padding: 20px; text-align: center; }}
                .content {{ background: #f9fafb; padding: 30px; }}
                .otp-box {{ background: white; border: 2px dashed #4F46E5; padding: 20px; text-align: center; font-size: 32px; font-weight: bold; letter-spacing: 8px; color: #4F46E5; margin: 20px 0; }}
                .footer {{ text-align: center; padding: 20px; color: #666; font-size: 12px; }}
            </style>
        </head>
        <body>
            <div class="container">
                <div class="header">
                    <h1>Kode OTP Login Anda</h1>
                </div>
                <div class="content">
                    <p>Halo <strong>{}</strong>,</p>
                    <p>Gunakan kode OTP berikut untuk menyelesaikan proses login Anda:</p>
                    <div class="otp-box">{}</div>
                    <p><strong>Kode ini berlaku selama 5 menit.</strong></p>
                    <p>Jangan bagikan kode ini kepada siapa pun, termasuk tim Big Auto.</p>
                    <p>Jika Anda tidak mencoba login, segera abaikan email ini dan hubungi kami.</p>
                </div>
                <div class="footer">
                    <p>Email otomatis, mohon tidak membalas.</p>
                    <p>&copy; 2025 Big Auto. All rights reserved.</p>
                </div>
            </div>
        </body>
        </html>
        "#,
        to_name, otp
    );

    send_email_via_resend(
        http_client,
        api_key,
        from_email,
        to_email,
        "Kode OTP Login Anda - Big Auto",
        &html_body
    ).await
}

// Internal helper function untuk mengirim email via Resend API
async fn send_email_via_resend(
    http_client: &reqwest::Client,
    api_key: &str,
    from_email: &str,
    to_email: &str,
    subject: &str,
    html_body: &str,
) -> Result<(), crate::error::AppError> {
    let request_body = json!({
        "from": from_email,
        "to": [to_email],
        "subject": subject,
        "html": html_body
    });

    tracing::debug!("Attempting to send email to {} via Resend API", to_email);

    let response = http_client
        .post("https://api.resend.com/emails")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| crate::error::AppError::email(format!("Failed to send request to Resend: {}", e)))?;

    if response.status().is_success() {
        tracing::info!("✅ Email sent successfully to {}", to_email);
        Ok(())
    } else {
        let error_text = response.text().await
            .map_err(|e| crate::error::AppError::email(format!("Failed to read error response: {}", e)))?;
        tracing::error!("❌ Failed to send email: {}", error_text);
        Err(crate::error::AppError::email(format!("Failed to send email: {}", error_text)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_email_config_from_env() {
        env::set_var("RESEND_API_KEY", "re_test_key");
        env::set_var("RESEND_FROM_EMAIL", "test@resend.dev");

        let config = EmailConfig::from_env().expect("Failed to load config");

        assert_eq!(config.resend_api_key, "re_test_key");
        assert_eq!(config.email_from, "test@resend.dev");
    }

    #[test]
    fn test_email_config_default_email() {
        env::set_var("RESEND_API_KEY", "re_test_key");
        env::remove_var("RESEND_FROM_EMAIL");

        let config = EmailConfig::from_env().expect("Failed to load config");

        assert_eq!(config.email_from, "onboarding@resend.dev", "Default email harus onboarding@resend.dev");
    }
}