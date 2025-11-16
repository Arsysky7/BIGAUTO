use crate::config::AppState;
use crate::models::{email_verification::EmailVerification, login_otp::LoginOtp, session::UserSession};
use std::time::Duration;

/// Background scheduler untuk cleanup expired data
pub struct CleanupScheduler {
    state: AppState,
}

impl CleanupScheduler {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    /// Start background cleanup tasks
    pub fn start(self) {
        // Spawn task untuk cleanup email verifications
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(3600)); // Every 1 hour

            loop {
                interval.tick().await;

                tracing::info!("🧹 Running background cleanup tasks...");

                // Cleanup expired email verifications
                match EmailVerification::cleanup_expired(&self.state.db).await {
                    Ok(deleted) => {
                        if deleted > 0 {
                            tracing::info!("✅ Cleaned up {} expired email verifications", deleted);
                        }
                    }
                    Err(e) => tracing::error!("❌ Failed to cleanup email verifications: {}", e),
                }

                // Cleanup expired OTPs
                match LoginOtp::cleanup_expired(&self.state.db).await {
                    Ok(deleted) => {
                        if deleted > 0 {
                            tracing::info!("✅ Cleaned up {} expired OTPs", deleted);
                        }
                    }
                    Err(e) => tracing::error!("❌ Failed to cleanup OTPs: {}", e),
                }

                // Cleanup expired sessions
                match UserSession::cleanup_expired(&self.state.db).await {
                    Ok(deleted) => {
                        if deleted > 0 {
                            tracing::info!("✅ Cleaned up {} expired sessions", deleted);
                        }
                    }
                    Err(e) => tracing::error!("❌ Failed to cleanup expired sessions: {}", e),
                }

                // Cleanup inactive sessions (30+ days)
                match UserSession::cleanup_inactive(&self.state.db).await {
                    Ok(deleted) => {
                        if deleted > 0 {
                            tracing::info!("✅ Cleaned up {} inactive sessions", deleted);
                        }
                    }
                    Err(e) => tracing::error!("❌ Failed to cleanup inactive sessions: {}", e),
                }

                tracing::info!("✅ Background cleanup tasks completed");
            }
        });
    }
}
