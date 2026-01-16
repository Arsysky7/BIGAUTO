use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use shared::utils::validation;
use sqlx::{PgPool, Row, FromRow};
use utoipa::{IntoParams, ToSchema};

use crate::{
    config::AppConfig,
    domain::review::{CreateReviewRequest, ReviewWithCustomer, SellerRatingSummary, RatingDistribution},
    error::AppError,
    middleware::{AuthUser, AuthSeller},
};

// Query parameters untuk pagination
#[derive(Debug, Deserialize, IntoParams)]
pub struct ReviewQueryParams {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_page() -> i64 { 1 }
fn default_limit() -> i64 { 10 }

// Response sukses untuk submit review
#[derive(Debug, Serialize, ToSchema)]
pub struct SubmitReviewResponse {
    pub message: String,
    pub review_id: i32,
}

// Submit review untuk seller setelah transaksi selesai
#[utoipa::path(
    post,
    path = "/api/sellers/{seller_id}/reviews",
    tag = "Ratings",
    security(("bearer_auth" = [])),
    request_body = CreateReviewRequest,
    params(
        ("seller_id" = i32, Path, description = "Seller ID")
    ),
    responses(
        (status = 200, description = "Review berhasil disubmit", body = SubmitReviewResponse),
        (status = 400, description = "Input tidak valid"),
        (status = 401, description = "Unauthorized"),
        (status = 422, description = "Spam prevention limit reached (production)"),
    )
)]
pub async fn submit_review(
    auth: AuthUser,
    Path(seller_id): Path<i32>,
    State(pool): State<PgPool>,
    State(config): State<AppConfig>,
    Json(req): Json<CreateReviewRequest>,
) -> Result<Json<SubmitReviewResponse>, AppError> {
    // Log untuk audit trail
    tracing::info!(
        "User {} ({}) submitting review for seller {}",
        auth.email,
        auth.role,
        seller_id
    );

    // Hanya customer yang bisa review seller
    if auth.role != "customer" {
        return Err(AppError::forbidden("Only customers can submit reviews"));
    }

    // Cek apakah customer pernah transaksi dengan seller
    let has_transaction = check_customer_transaction_history(&pool, auth.user_id, seller_id).await?;
    if !has_transaction {
        return Err(AppError::forbidden("You can only review sellers you have completed transactions with"));
    }

    // Cek apakah sudah pernah review seller ini
    let existing_review = check_existing_review(&pool, auth.user_id, seller_id).await?;
    if existing_review {
        return Err(AppError::bad_request("You have already reviewed this seller"));
    }

    // Strict validation: prevent spam reviews di production
    if config.strict_validation() {
        let today_reviews = count_user_reviews_today(&pool, auth.user_id).await?;
        const MAX_REVIEWS_PER_DAY: i64 = 10;

        if today_reviews >= MAX_REVIEWS_PER_DAY {
            return Err(AppError::validation(format!(
                "Maksimal {} reviews per hari (production mode untuk prevent spam)",
                MAX_REVIEWS_PER_DAY
            )));
        }
    }

    // Validasi overall rating wajib 1-5
    if !validation::is_valid_rating(req.overall_rating) {
        return Err(AppError::validation("Overall rating harus antara 1-5"));
    }

    // Validasi detail ratings jika ada
    if let Some(rating) = req.vehicle_condition_rating {
        if !validation::is_valid_rating(rating) {
            return Err(AppError::validation("Vehicle condition rating harus antara 1-5"));
        }
    }

    if let Some(rating) = req.accuracy_rating {
        if !validation::is_valid_rating(rating) {
            return Err(AppError::validation("Accuracy rating harus antara 1-5"));
        }
    }

    if let Some(rating) = req.service_rating {
        if !validation::is_valid_rating(rating) {
            return Err(AppError::validation("Service rating harus antara 1-5"));
        }
    }

    // Validasi comment length untuk prevent abuse
    if let Some(ref comment) = req.comment {
        if comment.len() > 1000 {
            return Err(AppError::validation("Comment maksimal 1000 karakter"));
        }
    }

    // Validasi jumlah photos
    if let Some(ref photos) = req.photos {
        if photos.len() > 3 {
            return Err(AppError::validation("Maksimal 3 foto per review"));
        }
    }

    // Insert review ke database dengan proper vehicle_id
    let review_id = insert_review(&pool, auth.user_id, seller_id, req).await?;

    Ok(Json(SubmitReviewResponse {
        message: "Review berhasil disubmit".to_string(),
        review_id,
    }))
}

// Ambil semua ratings untuk seller
#[utoipa::path(
    get,
    path = "/api/sellers/{seller_id}/ratings",
    tag = "Ratings",
    params(
        ("seller_id" = i32, Path, description = "Seller ID"),
        ReviewQueryParams
    ),
    responses(
        (status = 200, description = "List ratings berhasil diambil", body = Vec<ReviewWithCustomer>),
        (status = 404, description = "Seller tidak ditemukan"),
    )
)]
pub async fn get_seller_ratings(
    Path(seller_id): Path<i32>,
    Query(params): Query<ReviewQueryParams>,
    State(pool): State<PgPool>,
) -> Result<Json<Vec<ReviewWithCustomer>>, AppError> {
    let offset = (params.page - 1) * params.limit;
    let reviews = fetch_seller_reviews(&pool, seller_id, params.limit, offset).await?;
    Ok(Json(reviews))
}

// Ambil rating summary untuk seller (average, distribution, dll)
#[utoipa::path(
    get,
    path = "/api/sellers/{seller_id}/rating-summary",
    tag = "Ratings",
    params(
        ("seller_id" = i32, Path, description = "Seller ID")
    ),
    responses(
        (status = 200, description = "Rating summary berhasil diambil", body = SellerRatingSummary),
        (status = 404, description = "Seller tidak ditemukan"),
    )
)]
pub async fn get_seller_rating_summary(
    Path(seller_id): Path<i32>,
    State(pool): State<PgPool>,
) -> Result<Json<SellerRatingSummary>, AppError> {
    let summary = calculate_rating_summary(&pool, seller_id).await?;
    Ok(Json(summary))
}

// Seller melihat reviews mereka sendiri (seller-only endpoint)
#[utoipa::path(
    get,
    path = "/api/sellers/me/reviews",
    tag = "Ratings",
    security(("bearer_auth" = [])),
    params(
        ReviewQueryParams
    ),
    responses(
        (status = 200, description = "List reviews yang diterima seller", body = Vec<ReviewWithCustomer>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "User bukan seller"),
    )
)]
pub async fn get_my_seller_reviews(
    auth: AuthSeller,
    Query(params): Query<ReviewQueryParams>,
    State(pool): State<PgPool>,
) -> Result<Json<Vec<ReviewWithCustomer>>, AppError> {
    // Log untuk audit trail
    tracing::info!(
        "Seller {} viewing their reviews",
        auth.email
    );

    let offset = (params.page - 1) * params.limit;
    let reviews = fetch_seller_reviews(&pool, auth.user_id, params.limit, offset).await?;
    Ok(Json(reviews))
}

// === Helper Functions ===

// Count reviews yang dibuat user hari ini (untuk spam prevention)
async fn count_user_reviews_today(pool: &PgPool, customer_id: i32) -> Result<i64, AppError> {
    let result = sqlx::query(
        r#"
        SELECT COUNT(*) as count
        FROM reviews
        WHERE customer_id = $1
          AND created_at >= CURRENT_DATE::date
          AND created_at < CURRENT_DATE::date + INTERVAL '1 day'
        "#
    )
    .bind(customer_id)
    .fetch_one(pool)
    .await?;

    Ok(result.get::<i64, _>("count"))
}

// Fetch reviews seller dengan customer info
async fn fetch_seller_reviews(
    pool: &PgPool,
    seller_id: i32,
    limit: i64,
    offset: i64,
) -> Result<Vec<ReviewWithCustomer>, AppError> {
    #[derive(sqlx::FromRow)]
    struct ReviewRow {
        id: i32,
        overall_rating: i32,
        vehicle_condition_rating: Option<i32>,
        accuracy_rating: Option<i32>,
        service_rating: Option<i32>,
        comment: Option<String>,
        photos: Option<String>,
        created_at: chrono::DateTime<chrono::Utc>,
        customer_id: i32,
        customer_name: String,
        customer_photo: Option<String>,
    }

    let rows = sqlx::query(
        r#"
        SELECT
            r.id,
            r.overall_rating,
            r.vehicle_condition_rating,
            r.accuracy_rating,
            r.service_rating,
            r.comment,
            r.photos::text AS photos,
            r.created_at,
            r.customer_id,
            u.name AS customer_name,
            u.profile_photo AS customer_photo
        FROM reviews r
        INNER JOIN users u ON r.customer_id = u.id
        WHERE r.seller_id = $1 AND r.is_visible = true
        ORDER BY r.created_at DESC
        LIMIT $2 OFFSET $3
        "#
    )
    .bind(seller_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let mut reviews = Vec::new();
    for row in rows {
        let review_row = ReviewRow::from_row(&row)?;
        reviews.push(ReviewWithCustomer {
            id: review_row.id,
            overall_rating: review_row.overall_rating,
            vehicle_condition_rating: review_row.vehicle_condition_rating,
            accuracy_rating: review_row.accuracy_rating,
            service_rating: review_row.service_rating,
            comment: review_row.comment,
            photos: review_row.photos.and_then(|s| serde_json::from_str(&s).ok()),
            created_at: review_row.created_at,
            customer_id: review_row.customer_id,
            customer_name: review_row.customer_name,
            customer_photo: review_row.customer_photo,
        });
    }

    Ok(reviews)
}

// Hitung rating summary untuk seller
async fn calculate_rating_summary(
    pool: &PgPool,
    seller_id: i32,
) -> Result<SellerRatingSummary, AppError> {
    // Aggregate rating stats
    let result = sqlx::query(
        r#"
        SELECT
            COUNT(*)::BIGINT as total_reviews,
            COALESCE(AVG(overall_rating), 0) as average_rating,
            AVG(vehicle_condition_rating) as avg_vehicle_condition,
            AVG(accuracy_rating) as avg_accuracy,
            AVG(service_rating) as avg_service
        FROM reviews
        WHERE seller_id = $1 AND is_visible = true
        "#
    )
    .bind(seller_id)
    .fetch_optional(pool)
    .await?;

    let (total_reviews, average_rating, avg_vehicle, avg_accuracy, avg_service) = match result {
        Some(row) => (
            row.get::<i64, _>("total_reviews"),
            row.get::<f64, _>("average_rating"),
            row.get::<Option<f64>, _>("avg_vehicle_condition"),
            row.get::<Option<f64>, _>("avg_accuracy"),
            row.get::<Option<f64>, _>("avg_service")
        ),
        None => (0, 0.0, None, None, None),
    };

    // Rating distribution (1-5 star counts)
    let distribution = calculate_rating_distribution(pool, seller_id).await?;

    Ok(SellerRatingSummary {
        seller_id,
        total_reviews,
        average_rating: (average_rating * 10.0).round() / 10.0,
        rating_distribution: distribution,
        average_vehicle_condition: avg_vehicle.map(|v| (v * 10.0).round() / 10.0),
        average_accuracy: avg_accuracy.map(|v| (v * 10.0).round() / 10.0),
        average_service: avg_service.map(|v| (v * 10.0).round() / 10.0),
    })
}

// Hitung distribusi rating 1-5 bintang
async fn calculate_rating_distribution(
    pool: &PgPool,
    seller_id: i32,
) -> Result<RatingDistribution, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT overall_rating, COUNT(*)::BIGINT as count
        FROM reviews
        WHERE seller_id = $1 AND is_visible = true
        GROUP BY overall_rating
        "#
    )
    .bind(seller_id)
    .fetch_all(pool)
    .await?;

    let mut counts = Vec::new();
    for row in rows {
        let rating = row.get::<i32, _>("overall_rating");
        let count = row.get::<i64, _>("count");
        counts.push((rating, count));
    }

    let mut distribution = RatingDistribution {
        five_star: 0,
        four_star: 0,
        three_star: 0,
        two_star: 0,
        one_star: 0,
    };

    for (rating, count) in counts {
        match rating {
            5 => distribution.five_star = count,
            4 => distribution.four_star = count,
            3 => distribution.three_star = count,
            2 => distribution.two_star = count,
            1 => distribution.one_star = count,
            _ => {}
        }
    }

    Ok(distribution)
}

// Insert review baru ke database
async fn insert_review(
    pool: &PgPool,
    customer_id: i32,
    seller_id: i32,
    req: CreateReviewRequest,
) -> Result<i32, AppError> {
    // Serialize photos array ke JSONB
    let photos_json = req.photos.as_ref().map(|p| serde_json::to_value(p).unwrap());

    let result = sqlx::query(
        r#"
        INSERT INTO reviews (
            customer_id,
            seller_id,
            overall_rating,
            vehicle_condition_rating,
            accuracy_rating,
            service_rating,
            comment,
            photos,
            review_for_type
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'rental')
        RETURNING id
        "#
    )
    .bind(customer_id)
    .bind(seller_id)
    .bind(req.overall_rating)
    .bind(req.vehicle_condition_rating)
    .bind(req.accuracy_rating)
    .bind(req.service_rating)
    .bind(req.comment)
    .bind(photos_json)
    .fetch_one(pool)
    .await?;

    Ok(result.get::<i64, _>("id") as i32)
}

// Cek apakah customer pernah transaksi dengan seller (completed transactions only)
async fn check_customer_transaction_history(
    pool: &PgPool,
    customer_id: i32,
    seller_id: i32,
) -> Result<bool, AppError> {
    let result: Option<bool> = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM rental_bookings
            WHERE customer_id = $1 AND seller_id = $2 AND status = 'selesai'
            UNION
            SELECT 1 FROM sale_orders
            WHERE buyer_id = $1 AND seller_id = $2 AND status = 'completed'
        )
        "#
    )
    .bind(customer_id)
    .bind(seller_id)
    .fetch_optional(pool)
    .await?;

    Ok(result.unwrap_or(false))
}

// Cek apakah customer sudah pernah review seller ini
async fn check_existing_review(
    pool: &PgPool,
    customer_id: i32,
    seller_id: i32,
) -> Result<bool, AppError> {
    let result: Option<bool> = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM reviews WHERE customer_id = $1 AND seller_id = $2)"
    )
    .bind(customer_id)
    .bind(seller_id)
    .fetch_optional(pool)
    .await?;

    Ok(result.unwrap_or(false))
}
