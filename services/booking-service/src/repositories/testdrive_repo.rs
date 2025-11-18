use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use sqlx::types::JsonValue;

use crate::{
    domain::testdrive::{TestDriveBooking, CreateTestDriveRequest, TestDriveStatus},
    error::AppError,
};

// Create test drive booking baru
pub async fn create_testdrive(
    pool: &PgPool,
    customer_id: i32,
    seller_id: i32,
    payload: &CreateTestDriveRequest,
) -> Result<TestDriveBooking, AppError> {
    let timeout_at = Utc::now() + Duration::hours(2);

    let testdrive = sqlx::query_as(
        "INSERT INTO testdrive_bookings (
            vehicle_id, customer_id, seller_id,
            requested_date, requested_time,
            customer_name, customer_phone, customer_email,
            notes, status, timeout_at
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11
        ) RETURNING *"
    )
    .bind(payload.vehicle_id)
    .bind(customer_id)
    .bind(seller_id)
    .bind(payload.requested_date)
    .bind(&payload.requested_time)
    .bind(&payload.customer_name)
    .bind(&payload.customer_phone)
    .bind(&payload.customer_email)
    .bind(&payload.notes)
    .bind(TestDriveStatus::MenungguKonfirmasi.as_str())
    .bind(timeout_at)
    .fetch_one(pool)
    .await?;

    Ok(testdrive)
}

// Ambil test drive booking by ID
pub async fn find_testdrive_by_id(
    pool: &PgPool,
    id: i32,
) -> Result<Option<TestDriveBooking>, AppError> {
    let result = sqlx::query_as(
        "SELECT * FROM testdrive_bookings WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(result)
}

// Ambil test drive bookings by customer
pub async fn find_testdrives_by_customer(
    pool: &PgPool,
    customer_id: i32,
    status: Option<String>,
) -> Result<Vec<TestDriveBooking>, AppError> {
    let testdrives = if let Some(status_filter) = status {
        sqlx::query_as(
            "SELECT * FROM testdrive_bookings
             WHERE customer_id = $1 AND status = $2
             ORDER BY created_at DESC"
        )
        .bind(customer_id)
        .bind(status_filter)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as(
            "SELECT * FROM testdrive_bookings
             WHERE customer_id = $1
             ORDER BY created_at DESC"
        )
        .bind(customer_id)
        .fetch_all(pool)
        .await?
    };

    Ok(testdrives)
}

// Ambil test drive bookings by seller
pub async fn find_testdrives_by_seller(
    pool: &PgPool,
    seller_id: i32,
    status: Option<String>,
) -> Result<Vec<TestDriveBooking>, AppError> {
    let testdrives = if let Some(status_filter) = status {
        sqlx::query_as(
            "SELECT * FROM testdrive_bookings
             WHERE seller_id = $1 AND status = $2
             ORDER BY created_at DESC"
        )
        .bind(seller_id)
        .bind(status_filter)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as(
            "SELECT * FROM testdrive_bookings
             WHERE seller_id = $1
             ORDER BY created_at DESC"
        )
        .bind(seller_id)
        .fetch_all(pool)
        .await?
    };

    Ok(testdrives)
}

// Seller reschedule test drive dengan alternative slots
pub async fn reschedule_testdrive(
    pool: &PgPool,
    id: i32,
    reschedule_slots: JsonValue,
) -> Result<TestDriveBooking, AppError> {
    let timeout_at = Utc::now() + Duration::hours(2);

    let testdrive = sqlx::query_as(
        "UPDATE testdrive_bookings
         SET status = $4,
             reschedule_slots = $1,
             timeout_at = $2,
             updated_at = NOW()
         WHERE id = $3
         RETURNING *"
    )
    .bind(reschedule_slots)
    .bind(timeout_at)
    .bind(id)
    .bind(TestDriveStatus::SellerReschedule.as_str())
    .fetch_one(pool)
    .await?;

    Ok(testdrive)
}

// Customer pilih slot reschedule
pub async fn choose_reschedule_slot(
    pool: &PgPool,
    id: i32,
    slot_index: usize,
) -> Result<TestDriveBooking, AppError> {
    // Ambil testdrive dulu untuk get reschedule_slots
    let testdrive: TestDriveBooking = sqlx::query_as(
        "SELECT * FROM testdrive_bookings WHERE id = $1"
    )
    .bind(id)
    .fetch_one(pool)
    .await?;

    if testdrive.reschedule_slots.is_none() {
        return Err(AppError::bad_request("Tidak ada reschedule slots"));
    }

    let slots = testdrive.reschedule_slots.unwrap();
    let slots_array = slots.as_array()
        .ok_or_else(|| AppError::internal("Invalid reschedule_slots format"))?;

    if slot_index >= slots_array.len() {
        return Err(AppError::bad_request("Slot index tidak valid"));
    }

    let selected_slot = &slots_array[slot_index];
    let new_date = selected_slot.get("date")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::internal("Invalid slot date"))?;
    let new_time = selected_slot.get("time")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::internal("Invalid slot time"))?;

    let new_date_parsed = new_date.parse::<DateTime<Utc>>()
        .map_err(|_| AppError::internal("Invalid date format"))?;

    let updated = sqlx::query_as(
        "UPDATE testdrive_bookings
         SET requested_date = $1,
             requested_time = $2,
             status = $5,
             reschedule_slots = NULL,
             timeout_at = $3,
             updated_at = NOW()
         WHERE id = $4
         RETURNING *"
    )
    .bind(new_date_parsed)
    .bind(new_time)
    .bind(Utc::now() + Duration::hours(2))
    .bind(id)
    .bind(TestDriveStatus::MenungguKonfirmasi.as_str())
    .fetch_one(pool)
    .await?;

    Ok(updated)
}

// Seller confirm test drive
pub async fn confirm_testdrive(
    pool: &PgPool,
    id: i32,
) -> Result<TestDriveBooking, AppError> {
    let testdrive = sqlx::query_as(
        "UPDATE testdrive_bookings
         SET status = $2,
             timeout_at = NULL,
             updated_at = NOW()
         WHERE id = $1
         RETURNING *"
    )
    .bind(id)
    .bind(TestDriveStatus::Diterima.as_str())
    .fetch_one(pool)
    .await?;

    Ok(testdrive)
}

// Seller complete test drive
pub async fn complete_testdrive(
    pool: &PgPool,
    id: i32,
) -> Result<TestDriveBooking, AppError> {
    let testdrive = sqlx::query_as(
        "UPDATE testdrive_bookings
         SET status = $2,
             updated_at = NOW()
         WHERE id = $1
         RETURNING *"
    )
    .bind(id)
    .bind(TestDriveStatus::Selesai.as_str())
    .fetch_one(pool)
    .await?;

    Ok(testdrive)
}

// Cancel test drive booking
pub async fn cancel_testdrive(
    pool: &PgPool,
    id: i32,
    cancel_reason: &str,
) -> Result<TestDriveBooking, AppError> {
    let testdrive = sqlx::query_as(
        "UPDATE testdrive_bookings
         SET status = $3,
             cancel_reason = $1,
             cancelled_at = NOW(),
             updated_at = NOW()
         WHERE id = $2
         RETURNING *"
    )
    .bind(cancel_reason)
    .bind(id)
    .bind(TestDriveStatus::Cancelled.as_str())
    .fetch_one(pool)
    .await?;

    Ok(testdrive)
}

// Auto-timeout test drive bookings yang sudah lewat 2 jam
pub async fn timeout_expired_testdrives(pool: &PgPool) -> Result<i64, AppError> {
    let result = sqlx::query(
        "UPDATE testdrive_bookings
         SET status = $3, updated_at = NOW()
         WHERE status IN ($1, $2)
           AND timeout_at < NOW()"
    )
    .bind(TestDriveStatus::MenungguKonfirmasi.as_str())
    .bind(TestDriveStatus::SellerReschedule.as_str())
    .bind(TestDriveStatus::Timeout.as_str())
    .execute(pool)
    .await?;

    Ok(result.rows_affected() as i64)
}
