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

                // Cleanup expired email verifications (with retry logic)
                let db = self.state.db.clone();
                tokio::spawn(async move {
                    for attempt in 1..=3 {
                        match EmailVerification::cleanup_expired(&db).await {
                            Ok(deleted) => {
                                if deleted > 0 {
                                    tracing::info!("✅ Cleaned up {} expired email verifications", deleted);
                                }
                                break;
                            }
                            Err(e) => {
                                if attempt == 3 {
                                    tracing::error!("❌ Failed to cleanup email verifications after 3 attempts: {}", e);
                                } else {
                                    tokio::time::sleep(Duration::from_millis(1000)).await;
                                }
                            }
                        }
                    }
                });

                // Cleanup expired OTPs (with retry logic)
                let db = self.state.db.clone();
                tokio::spawn(async move {
                    for attempt in 1..=3 {
                        match LoginOtp::cleanup_expired(&db).await {
                            Ok(deleted) => {
                                if deleted > 0 {
                                    tracing::info!("✅ Cleaned up {} expired OTPs", deleted);
                                }
                                break;
                            }
                            Err(e) => {
                                if attempt == 3 {
                                    tracing::error!("❌ Failed to cleanup OTPs after 3 attempts: {}", e);
                                } else {
                                    tokio::time::sleep(Duration::from_millis(1000)).await;
                                }
                            }
                        }
                    }
                });

                // Cleanup expired sessions (with retry logic)
                let db = self.state.db.clone();
                tokio::spawn(async move {
                    for attempt in 1..=3 {
                        match UserSession::cleanup_expired(&db).await {
                            Ok(deleted) => {
                                if deleted > 0 {
                                    tracing::info!("✅ Cleaned up {} expired sessions", deleted);
                                }
                                break;
                            }
                            Err(e) => {
                                if attempt == 3 {
                                    tracing::error!("❌ Failed to cleanup expired sessions after 3 attempts: {}", e);
                                } else {
                                    tokio::time::sleep(Duration::from_millis(1000)).await;
                                }
                            }
                        }
                    }
                });

                // Cleanup inactive sessions (with retry logic)
                let db = self.state.db.clone();
                tokio::spawn(async move {
                    for attempt in 1..=3 {
                        match UserSession::cleanup_inactive(&db).await {
                            Ok(deleted) => {
                                if deleted > 0 {
                                    tracing::info!("✅ Cleaned up {} inactive sessions", deleted);
                                }
                                break;
                            }
                            Err(e) => {
                                if attempt == 3 {
                                    tracing::error!("❌ Failed to cleanup inactive sessions after 3 attempts: {}", e);
                                } else {
                                    tokio::time::sleep(Duration::from_millis(1000)).await;
                                }
                            }
                        }
                    }
                });

                tracing::info!("✅ Background cleanup tasks completed");
            }
        });
    }
}
