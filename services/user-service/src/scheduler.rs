use crate::config::AppState;
use crate::domain::user::User;
use crate::domain::review::Review;
use crate::domain::favorite::Favorite;
use std::time::Duration;

/// Background scheduler for user service cleanup and maintenance
pub struct UserScheduler {
    state: AppState,
}

impl UserScheduler {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    /// Start background cleanup tasks for user service
    pub fn start(self) {
        // Check if scheduler is disabled
        if std::env::var("DISABLE_SCHEDULER").unwrap_or_else(|_| "false".to_string()) == "true" {
            tracing::info!("👤 User scheduler disabled via DISABLE_SCHEDULER environment variable");
            return;
        }

        tracing::info!("👤 Starting User Service Background Scheduler...");

        // Spawn task for user maintenance tasks
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(3600)); // Every 1 hour

            loop {
                interval.tick().await;

                tracing::info!("🧹 Running user service cleanup tasks...");

                // Cleanup expired email verification tokens
                let db = self.state.db.clone();
                tokio::spawn(async move {
                    for attempt in 1..=3 {
                        match User::cleanup_expired_verifications(&db).await {
                            Ok(deleted) => {
                                if deleted > 0 {
                                    tracing::info!("✅ Cleaned up {} expired email verification tokens", deleted);
                                }
                                break;
                            }
                            Err(e) => {
                                if attempt == 3 {
                                    tracing::error!("❌ Failed to cleanup expired verifications after 3 attempts: {}", e);
                                } else {
                                    tokio::time::sleep(Duration::from_millis(1000)).await;
                                }
                            }
                        }
                    }
                });

                // Cleanup spam/inappropriate reviews
                let db = self.state.db.clone();
                tokio::spawn(async move {
                    for attempt in 1..=3 {
                        match Review::cleanup_spam_reviews(&db).await {
                            Ok(deleted) => {
                                if deleted > 0 {
                                    tracing::info!("✅ Cleaned up {} spam/inappropriate reviews", deleted);
                                }
                                break;
                            }
                            Err(e) => {
                                if attempt == 3 {
                                    tracing::error!("❌ Failed to cleanup spam reviews after 3 attempts: {}", e);
                                } else {
                                    tokio::time::sleep(Duration::from_millis(1000)).await;
                                }
                            }
                        }
                    }
                });

                // Cleanup orphaned favorites (for deleted vehicles)
                let db = self.state.db.clone();
                tokio::spawn(async move {
                    for attempt in 1..=3 {
                        match Favorite::cleanup_orphaned_favorites(&db).await {
                            Ok(deleted) => {
                                if deleted > 0 {
                                    tracing::info!("✅ Cleaned up {} orphaned favorites", deleted);
                                }
                                break;
                            }
                            Err(e) => {
                                if attempt == 3 {
                                    tracing::error!("❌ Failed to cleanup orphaned favorites after 3 attempts: {}", e);
                                } else {
                                    tokio::time::sleep(Duration::from_millis(1000)).await;
                                }
                            }
                        }
                    }
                });

                // Deactivate inactive users (not logged in for 365 days)
                let db = self.state.db.clone();
                tokio::spawn(async move {
                    for attempt in 1..=3 {
                        match User::deactivate_inactive_users(&db).await {
                            Ok(deactivated) => {
                                if deactivated > 0 {
                                    tracing::info!("✅ Deactivated {} inactive users", deactivated);
                                }
                                break;
                            }
                            Err(e) => {
                                if attempt == 3 {
                                    tracing::error!("❌ Failed to deactivate inactive users after 3 attempts: {}", e);
                                } else {
                                    tokio::time::sleep(Duration::from_millis(1000)).await;
                                }
                            }
                        }
                    }
                });

                tracing::info!("✅ User service cleanup tasks completed");
            }
        });
    }
}