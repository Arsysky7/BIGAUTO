use lettre::{
    message::{Message, MultiPart, SinglePart},
    transport::smtp::authentication::Credentials,
    SmtpTransport, Transport,
};
use std::env;

pub struct EmailConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub email_from: String,
    pub email_from_name: String,
}

impl EmailConfig {
    // Load konfigurasi email dari environment variables
    pub fn from_env() -> Result<Self, crate::error::AppError> {
        Ok(EmailConfig {
            smtp_host: env::var("SMTP_HOST").map_err(|_| crate::error::AppError::email("SMTP_HOST tidak ditemukan"))?,
            smtp_port: env::var("SMTP_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(587),
            smtp_username: env::var("SMTP_USERNAME").map_err(|_| crate::error::AppError::email("SMTP_USERNAME tidak ditemukan"))?,
            smtp_password: env::var("SMTP_PASSWORD").map_err(|_| crate::error::AppError::email("SMTP_PASSWORD tidak ditemukan"))?,
            email_from: env::var("EMAIL_FROM").map_err(|_| crate::error::AppError::email("EMAIL_FROM tidak ditemukan"))?,
            email_from_name: env::var("EMAIL_FROM_NAME").unwrap_or_else(|_| "Big Auto".to_string()),
        })
    }
}

// Kirim email verifikasi dengan link aktivasi akun
pub fn send_verification_email(
    to_email: &str,
    to_name: &str,
    verification_token: &str,
) -> Result<(), crate::error::AppError> {
    let config = EmailConfig::from_env()?;
    let verification_link = format!(
        "{}/api/auth/verify-email?token={}",
        env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:3000".to_string()),
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

    send_email(&config, to_email, "Verifikasi Email Anda - Big Auto", &html_body)
}

// Kirim OTP untuk login melalui email
pub fn send_otp_email(
    to_email: &str,
    to_name: &str,
    otp: &str,
) -> Result<(), crate::error::AppError> {
    let config = EmailConfig::from_env()?;

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

    send_email(&config, to_email, "Kode OTP Login Anda - Big Auto", &html_body)
}

// Internal helper function untuk mengirim email via SMTP
fn send_email(
    config: &EmailConfig,
    to_email: &str,
    subject: &str,
    html_body: &str,
) -> Result<(), crate::error::AppError> {
    let from_address = format!("{} <{}>", config.email_from_name, config.email_from);

    // Create plain text version (strip HTML tags untuk fallback)
    let plain_text = html_body
        .replace("<br>", "\n")
        .replace("</p>", "\n\n")
        .replace("<strong>", "")
        .replace("</strong>", "")
        .replace("<h1>", "")
        .replace("</h1>", "\n")
        .chars()
        .filter(|c| *c != '<' && *c != '>')
        .collect::<String>();

    // Build multipart email dengan HTML + plain text alternative (best practice)
    let email = Message::builder()
        .from(from_address.parse().map_err(|e| crate::error::AppError::email(format!("Invalid from address: {}", e)))?)
        .to(to_email.parse().map_err(|e| crate::error::AppError::email(format!("Invalid to address: {}", e)))?)
        .subject(subject)
        .multipart(
            MultiPart::alternative()
                .singlepart(SinglePart::plain(plain_text))
                .singlepart(SinglePart::html(html_body.to_string()))
        )
        .map_err(|e| crate::error::AppError::email(format!("Failed to build email: {}", e)))?;

    let credentials = Credentials::new(
        config.smtp_username.clone(),
        config.smtp_password.clone(),
    );

    // Build SMTP transport with explicit STARTTLS and timeout
    let mailer = SmtpTransport::starttls_relay(&config.smtp_host)
        .map_err(|e| crate::error::AppError::email(format!("Failed to create SMTP transport: {}", e)))?
        .credentials(credentials)
        .port(config.smtp_port)
        .timeout(Some(std::time::Duration::from_secs(30)))
        .build();

    tracing::debug!("Attempting to send HTML email to {} via {}", to_email, config.smtp_host);

    mailer
        .send(&email)
        .map_err(|e| crate::error::AppError::email(format!("Failed to send email: {}", e)))?;

    tracing::info!("✅ Email sent successfully to {}", to_email);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_config_from_env() {
        env::set_var("SMTP_HOST", "smtp.gmail.com");
        env::set_var("SMTP_PORT", "587");
        env::set_var("SMTP_USERNAME", "test@gmail.com");
        env::set_var("SMTP_PASSWORD", "password123");
        env::set_var("EMAIL_FROM", "test@gmail.com");
        env::set_var("EMAIL_FROM_NAME", "Test App");

        let config = EmailConfig::from_env().expect("Failed to load config");

        assert_eq!(config.smtp_host, "smtp.gmail.com");
        assert_eq!(config.smtp_port, 587);
        assert_eq!(config.smtp_username, "test@gmail.com");
    }

    #[test]
    fn test_email_config_default_port() {
        env::set_var("SMTP_HOST", "smtp.gmail.com");
        env::remove_var("SMTP_PORT");
        env::set_var("SMTP_USERNAME", "test@gmail.com");
        env::set_var("SMTP_PASSWORD", "password123");
        env::set_var("EMAIL_FROM", "test@gmail.com");

        let config = EmailConfig::from_env().expect("Failed to load config");

        assert_eq!(config.smtp_port, 587, "Default port harus 587");
    }
}
