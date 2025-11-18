use crate::config::AppState;
use crate::domain::vehicle::Vehicle;
use std::time::Duration;

/// Background scheduler for vehicle service cleanup and maintenance
pub struct VehicleScheduler {
    state: AppState,
}

impl VehicleScheduler {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    /// Start background cleanup tasks for vehicle service
    pub fn start(self) {
        // Check if scheduler is disabled
        if std::env::var("DISABLE_SCHEDULER").unwrap_or_else(|_| "false".to_string()) == "true" {
            tracing::info!("🚗 Vehicle scheduler disabled via DISABLE_SCHEDULER environment variable");
            return;
        }

        tracing::info!("🚗 Starting Vehicle Service Background Scheduler...");

        // Spawn task for vehicle maintenance tasks
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1800)); // Every 30 minutes

            loop {
                interval.tick().await;

                tracing::info!("🧹 Running vehicle service cleanup tasks...");

                // Cleanup expired vehicle listings
                let db = self.state.db.clone();
                tokio::spawn(async move {
                    for attempt in 1..=3 {
                        match Vehicle::cleanup_expired_listings(&db).await {
                            Ok(deleted) => {
                                if deleted > 0 {
                                    tracing::info!("✅ Cleaned up {} expired vehicle listings", deleted);
                                }
                                break;
                            }
                            Err(e) => {
                                if attempt == 3 {
                                    tracing::error!("❌ Failed to cleanup expired listings after 3 attempts: {}", e);
                                } else {
                                    tokio::time::sleep(Duration::from_millis(1000)).await;
                                }
                            }
                        }
                    }
                });

                // Update vehicle status for expired listings
                let db = self.state.db.clone();
                tokio::spawn(async move {
                    for attempt in 1..=3 {
                        match Vehicle::update_expired_status(&db).await {
                            Ok(updated) => {
                                if updated > 0 {
                                    tracing::info!("✅ Updated status for {} expired vehicles", updated);
                                }
                                break;
                            }
                            Err(e) => {
                                if attempt == 3 {
                                    tracing::error!("❌ Failed to update expired vehicle status after 3 attempts: {}", e);
                                } else {
                                    tokio::time::sleep(Duration::from_millis(1000)).await;
                                }
                            }
                        }
                    }
                });

                // Cleanup inactive vehicles (not updated in 90 days)
                let db = self.state.db.clone();
                tokio::spawn(async move {
                    for attempt in 1..=3 {
                        match Vehicle::cleanup_inactive_vehicles(&db).await {
                            Ok(deleted) => {
                                if deleted > 0 {
                                    tracing::info!("✅ Cleaned up {} inactive vehicles", deleted);
                                }
                                break;
                            }
                            Err(e) => {
                                if attempt == 3 {
                                    tracing::error!("❌ Failed to cleanup inactive vehicles after 3 attempts: {}", e);
                                } else {
                                    tokio::time::sleep(Duration::from_millis(1000)).await;
                                }
                            }
                        }
                    }
                });

  

                tracing::info!("✅ Vehicle service cleanup tasks completed");
            }
        });
    }

  
}