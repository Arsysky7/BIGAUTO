use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::{
    domain::rental::{RentalBooking, CreateRentalRequest, RentalStatus},
    error::AppError,
};

// Generate unique order ID untuk rental
async fn generate_rental_order_id(pool: &PgPool) -> Result<String, AppError> {
    loop {
        let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
        let random: i32 = rand::random::<i32>().abs() % 10000;
        let order_id = format!("RENT-{}-{:04}", timestamp, random);

        // Validasi unique
        let exists: (bool,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM rental_bookings WHERE order_id = $1)"
        )
        .bind(&order_id)
        .fetch_one(pool)
        .await?;

        if !exists.0 {
            return Ok(order_id);
        }
        // If exists, loop again and generate new ID
    }
}

// Create rental booking baru
pub async fn create_rental(
    pool: &PgPool,
    customer_id: i32,
    seller_id: i32,
    price_per_day: f64,
    payload: &CreateRentalRequest,
) -> Result<RentalBooking, AppError> {
    let order_id = generate_rental_order_id(pool).await?;

    let total_days = (payload.return_date - payload.pickup_date).num_days();
    if total_days <= 0 {
        return Err(AppError::validation("Tanggal return harus setelah pickup"));
    }

    let total_price = price_per_day * total_days as f64;

    let rental = sqlx::query_as(
        "INSERT INTO rental_bookings (
            vehicle_id, customer_id, seller_id, order_id,
            pickup_date, return_date,
            customer_name, customer_phone, customer_email,
            total_days, price_per_day, total_price, notes, status
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14
        ) RETURNING *"
    )
    .bind(payload.vehicle_id)
    .bind(customer_id)
    .bind(seller_id)
    .bind(&order_id)
    .bind(payload.pickup_date)
    .bind(payload.return_date)
    .bind(&payload.customer_name)
    .bind(&payload.customer_phone)
    .bind(&payload.customer_email)
    .bind(total_days as i32)
    .bind(price_per_day)
    .bind(total_price)
    .bind(&payload.notes)
    .bind(RentalStatus::PendingPayment.as_str())
    .fetch_one(pool)
    .await?;

    Ok(rental)
}

// Ambil rental booking by ID
pub async fn find_rental_by_id(
    pool: &PgPool,
    id: i32,
) -> Result<Option<RentalBooking>, AppError> {
    let result = sqlx::query_as(
        "SELECT * FROM rental_bookings WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(result)
}

// Ambil rental bookings by customer
pub async fn find_rentals_by_customer(
    pool: &PgPool,
    customer_id: i32,
    status: Option<String>,
) -> Result<Vec<RentalBooking>, AppError> {
    let rentals = if let Some(status_filter) = status {
        sqlx::query_as(
            "SELECT * FROM rental_bookings
             WHERE customer_id = $1 AND status = $2
             ORDER BY created_at DESC"
        )
        .bind(customer_id)
        .bind(status_filter)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as(
            "SELECT * FROM rental_bookings
             WHERE customer_id = $1
             ORDER BY created_at DESC"
        )
        .bind(customer_id)
        .fetch_all(pool)
        .await?
    };

    Ok(rentals)
}

// Ambil rental bookings by seller
pub async fn find_rentals_by_seller(
    pool: &PgPool,
    seller_id: i32,
    status: Option<String>,
) -> Result<Vec<RentalBooking>, AppError> {
    let rentals = if let Some(status_filter) = status {
        sqlx::query_as(
            "SELECT * FROM rental_bookings
             WHERE seller_id = $1 AND status = $2
             ORDER BY created_at DESC"
        )
        .bind(seller_id)
        .bind(status_filter)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as(
            "SELECT * FROM rental_bookings
             WHERE seller_id = $1
             ORDER BY created_at DESC"
        )
        .bind(seller_id)
        .fetch_all(pool)
        .await?
    };

    Ok(rentals)
}

// Update rental status
pub async fn update_rental_status(
    pool: &PgPool,
    id: i32,
    status: &str,
) -> Result<RentalBooking, AppError> {
    let rental = sqlx::query_as(
        "UPDATE rental_bookings
         SET status = $1, updated_at = NOW()
         WHERE id = $2
         RETURNING *"
    )
    .bind(status)
    .bind(id)
    .fetch_one(pool)
    .await?;

    Ok(rental)
}

// Validate pickup (seller confirms pickup dengan KTP)
pub async fn validate_pickup(
    pool: &PgPool,
    id: i32,
    ktp_photo: &str,
) -> Result<RentalBooking, AppError> {
    let rental = sqlx::query_as(
        "UPDATE rental_bookings
         SET actual_pickup_at = NOW(),
             ktp_photo = $1,
             status = 'berjalan',
             updated_at = NOW()
         WHERE id = $2
         RETURNING *"
    )
    .bind(ktp_photo)
    .bind(id)
    .fetch_one(pool)
    .await?;

    Ok(rental)
}

// Validate return (seller confirms return)
pub async fn validate_return(
    pool: &PgPool,
    id: i32,
) -> Result<RentalBooking, AppError> {
    let rental = sqlx::query_as(
        "UPDATE rental_bookings
         SET actual_return_at = NOW(),
             status = 'selesai',
             updated_at = NOW()
         WHERE id = $2
         RETURNING *"
    )
    .bind(id)
    .fetch_one(pool)
    .await?;

    Ok(rental)
}

// Cancel rental booking
pub async fn cancel_rental(
    pool: &PgPool,
    id: i32,
    cancel_reason: &str,
) -> Result<RentalBooking, AppError> {
    let rental = sqlx::query_as(
        "UPDATE rental_bookings
         SET status = 'cancelled',
             cancel_reason = $1,
             cancelled_at = NOW(),
             updated_at = NOW()
         WHERE id = $2
         RETURNING *"
    )
    .bind(cancel_reason)
    .bind(id)
    .fetch_one(pool)
    .await?;

    Ok(rental)
}

// Check apakah vehicle available untuk booking di range tanggal tertentu
pub async fn check_vehicle_availability(
    pool: &PgPool,
    vehicle_id: i32,
    pickup_date: DateTime<Utc>,
    return_date: DateTime<Utc>,
) -> Result<bool, AppError> {
    let conflict: (bool,) = sqlx::query_as(
        "SELECT EXISTS(
            SELECT 1 FROM rental_bookings
            WHERE vehicle_id = $1
              AND status NOT IN ('cancelled', 'selesai')
              AND (
                  (pickup_date <= $2 AND return_date >= $2) OR
                  (pickup_date <= $3 AND return_date >= $3) OR
                  (pickup_date >= $2 AND return_date <= $3)
              )
        )"
    )
    .bind(vehicle_id)
    .bind(pickup_date)
    .bind(return_date)
    .fetch_one(pool)
    .await?;

    Ok(!conflict.0)
}
