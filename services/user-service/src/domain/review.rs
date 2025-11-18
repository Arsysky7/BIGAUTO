use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;


// Request create review
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[schema(example = json!({
    "overall_rating": 5,
    "vehicle_condition_rating": 5,
    "accuracy_rating": 5,
    "service_rating": 5,
    "comment": "Pelayanan sangat baik, mobil bersih dan sesuai deskripsi!",
    "photos": ["https://res.cloudinary.com/drjf5hd0p/image/upload/v1234/reviews/review-1.jpg"]
}))]
pub struct CreateReviewRequest {
    #[schema(example = 5, minimum = 1, maximum = 5)]
    pub overall_rating: i32,
    #[schema(example = 5, minimum = 1, maximum = 5)]
    pub vehicle_condition_rating: Option<i32>,
    #[schema(example = 5, minimum = 1, maximum = 5)]
    pub accuracy_rating: Option<i32>,
    #[schema(example = 5, minimum = 1, maximum = 5)]
    pub service_rating: Option<i32>,
    #[schema(example = "Pelayanan sangat baik, mobil bersih dan sesuai deskripsi!")]
    pub comment: Option<String>,
    #[schema(example = json!(["https://res.cloudinary.com/drjf5hd0p/image/upload/v1234/reviews/review-1.jpg"]))]
    pub photos: Option<Vec<String>>,
}

// Response review dengan info customer
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "id": 1,
    "overall_rating": 5,
    "vehicle_condition_rating": 5,
    "accuracy_rating": 5,
    "service_rating": 5,
    "comment": "Pelayanan sangat baik, mobil bersih!",
    "photos": ["https://res.cloudinary.com/drjf5hd0p/image/upload/v1234/reviews/review-1.jpg"],
    "created_at": "2025-01-01T00:00:00Z",
    "customer_id": 123,
    "customer_name": "John Doe",
    "customer_photo": "https://res.cloudinary.com/drjf5hd0p/image/upload/v1234/profiles/user-123.jpg"
}))]
pub struct ReviewWithCustomer {
    pub id: i32,
    pub overall_rating: i32,
    pub vehicle_condition_rating: Option<i32>,
    pub accuracy_rating: Option<i32>,
    pub service_rating: Option<i32>,
    pub comment: Option<String>,
    pub photos: Option<Vec<String>>,
    pub created_at: DateTime<Utc>,
    // Info customer
    pub customer_id: i32,
    pub customer_name: String,
    pub customer_photo: Option<String>,
}

// Response rating summary untuk seller
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "seller_id": 1,
    "total_reviews": 150,
    "average_rating": 4.8,
    "rating_distribution": {
        "five_star": 120,
        "four_star": 20,
        "three_star": 5,
        "two_star": 3,
        "one_star": 2
    },
    "average_vehicle_condition": 4.9,
    "average_accuracy": 4.8,
    "average_service": 4.7
}))]
pub struct SellerRatingSummary {
    pub seller_id: i32,
    pub total_reviews: i64,
    pub average_rating: f64,
    pub rating_distribution: RatingDistribution,
    pub average_vehicle_condition: Option<f64>,
    pub average_accuracy: Option<f64>,
    pub average_service: Option<f64>,
}

// Distribusi rating 1-5 bintang
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "five_star": 120,
    "four_star": 20,
    "three_star": 5,
    "two_star": 3,
    "one_star": 2
}))]
pub struct RatingDistribution {
    pub five_star: i64,
    pub four_star: i64,
    pub three_star: i64,
    pub two_star: i64,
    pub one_star: i64,
}

// Review implementation for cleanup operations
pub struct Review;

impl Review {
    /// Cleanup spam/inappropriate reviews (flagged for more than 7 days)
    pub async fn cleanup_spam_reviews(pool: &PgPool) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            "DELETE FROM reviews WHERE is_inappropriate = true AND updated_at < NOW() - INTERVAL '7 days'"
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }
}

