use crate::config::AppState;
use crate::domain::rental::RentalBooking;
use std::time::Duration;

/// Background scheduler for booking service cleanup and maintenance
pub struct BookingScheduler {
    state: AppState,
}

impl BookingScheduler {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    /// Start background cleanup tasks for booking service
    pub fn start(self) {
        // Check if scheduler is disabled
        if std::env::var("DISABLE_SCHEDULER").unwrap_or_else(|_| "false".to_string()) == "true" {
            tracing::info!("🚗 Booking scheduler disabled via DISABLE_SCHEDULER environment variable");
            return;
        }

        tracing::info!("🚗 Starting Booking Service Background Scheduler...");

        // Spawn task for booking maintenance tasks
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(2400)); // Every 40 minutes

            loop {
                interval.tick().await;

                tracing::info!("🧹 Running booking service cleanup tasks...");

                // Cancel expired pending payment bookings (older than 1 hour)
                let db = self.state.db.clone();
                tokio::spawn(async move {
                    for attempt in 1..=3 {
                        match RentalBooking::cleanup_expired_pending_payments(&db).await {
                            Ok(cancelled) => {
                                if cancelled > 0 {
                                    tracing::info!("✅ Cancelled {} expired pending payment bookings", cancelled);
                                }
                                break;
                            }
                            Err(e) => {
                                if attempt == 3 {
                                    tracing::error!("❌ Failed to cleanup expired pending payments after 3 attempts: {}", e);
                                } else {
                                    tokio::time::sleep(Duration::from_millis(1000)).await;
                                }
                            }
                        }
                    }
                });

                // Auto-complete overdue rentals (where return date has passed)
                let db = self.state.db.clone();
                tokio::spawn(async move {
                    for attempt in 1..=3 {
                        match RentalBooking::auto_complete_overdue_rentals(&db).await {
                            Ok(completed) => {
                                if completed > 0 {
                                    tracing::info!("✅ Auto-completed {} overdue rentals", completed);
                                }
                                break;
                            }
                            Err(e) => {
                                if attempt == 3 {
                                    tracing::error!("❌ Failed to auto-complete overdue rentals after 3 attempts: {}", e);
                                } else {
                                    tokio::time::sleep(Duration::from_millis(1000)).await;
                                }
                            }
                        }
                    }
                });

                // Update booking status from "akan datang" to "berjalan" when pickup date arrives
                let db = self.state.db.clone();
                tokio::spawn(async move {
                    for attempt in 1..=3 {
                        match RentalBooking::update_pickup_ready_bookings(&db).await {
                            Ok(updated) => {
                                if updated > 0 {
                                    tracing::info!("✅ Updated {} bookings to 'berjalan' status", updated);
                                }
                                break;
                            }
                            Err(e) => {
                                if attempt == 3 {
                                    tracing::error!("❌ Failed to update pickup ready bookings after 3 attempts: {}", e);
                                } else {
                                    tokio::time::sleep(Duration::from_millis(1000)).await;
                                }
                            }
                        }
                    }
                });

                // Cleanup very old completed/cancelled bookings (older than 180 days)
                let db = self.state.db.clone();
                tokio::spawn(async move {
                    for attempt in 1..=3 {
                        match RentalBooking::cleanup_old_bookings(&db).await {
                            Ok(deleted) => {
                                if deleted > 0 {
                                    tracing::info!("✅ Cleaned up {} old bookings", deleted);
                                }
                                break;
                            }
                            Err(e) => {
                                if attempt == 3 {
                                    tracing::error!("❌ Failed to cleanup old bookings after 3 attempts: {}", e);
                                } else {
                                    tokio::time::sleep(Duration::from_millis(1000)).await;
                                }
                            }
                        }
                    }
                });

                tracing::info!("✅ Booking service cleanup tasks completed");
            }
        });
    }
}