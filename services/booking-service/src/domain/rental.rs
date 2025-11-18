use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// Model utama RentalBooking dari database
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RentalBooking {
    pub id: i32,
    pub vehicle_id: i32,
    pub customer_id: i32,
    pub seller_id: i32,
    pub order_id: String,
    pub pickup_date: DateTime<Utc>,
    pub return_date: DateTime<Utc>,
    pub actual_pickup_at: Option<DateTime<Utc>>,
    pub actual_return_at: Option<DateTime<Utc>>,
    pub customer_name: String,
    pub customer_phone: String,
    pub customer_email: String,
    pub ktp_photo: Option<String>,
    pub total_days: i32,
    pub price_per_day: f64,
    pub total_price: f64,
    pub notes: Option<String>,
    pub status: String,
    pub cancel_reason: Option<String>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Enum untuk status rental booking
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RentalStatus {
    PendingPayment,
    Paid,
    AkanDatang,
    Berjalan,
    Selesai,
    Cancelled,
}

impl RentalStatus {
    pub fn as_str(&self) -> &str {
        match self {
            RentalStatus::PendingPayment => "pending_payment",
            RentalStatus::Paid => "paid",
            RentalStatus::AkanDatang => "akan_datang",
            RentalStatus::Berjalan => "berjalan",
            RentalStatus::Selesai => "selesai",
            RentalStatus::Cancelled => "cancelled",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending_payment" => Some(RentalStatus::PendingPayment),
            "paid" => Some(RentalStatus::Paid),
            "akan_datang" => Some(RentalStatus::AkanDatang),
            "berjalan" => Some(RentalStatus::Berjalan),
            "selesai" => Some(RentalStatus::Selesai),
            "cancelled" => Some(RentalStatus::Cancelled),
            _ => None,
        }
    }
}

// Request untuk create rental booking baru
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateRentalRequest {
    #[schema(example = 1)]
    pub vehicle_id: i32,
    #[schema(example = "2025-12-01T10:00:00Z")]
    pub pickup_date: DateTime<Utc>,
    #[schema(example = "2025-12-05T10:00:00Z")]
    pub return_date: DateTime<Utc>,
    #[schema(example = "John Doe")]
    pub customer_name: String,
    #[schema(example = "081234567890")]
    pub customer_phone: String,
    #[schema(example = "john@example.com")]
    pub customer_email: String,
    #[schema(example = "Butuh mobil untuk liburan keluarga")]
    pub notes: Option<String>,
}

// Request untuk update status rental booking
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateRentalStatusRequest {
    #[schema(example = "paid")]
    pub status: String,
}

// Request untuk validate pickup (seller)
#[derive(Debug, Deserialize, ToSchema)]
pub struct ValidatePickupRequest {
    #[schema(example = "https://cloudinary.com/ktp.jpg")]
    pub ktp_photo: String,
}

// Request untuk validate return (seller)
#[derive(Debug, Deserialize, ToSchema)]
pub struct ValidateReturnRequest {
    #[schema(example = "Mobil dikembalikan dalam kondisi baik")]
    pub notes: Option<String>,
}

// Request untuk cancel rental booking
#[derive(Debug, Deserialize, ToSchema)]
pub struct CancelRentalRequest {
    #[schema(example = "Perubahan rencana perjalanan")]
    pub cancel_reason: String,
}

// Response untuk rental booking
#[derive(Debug, Serialize, ToSchema)]
pub struct RentalBookingResponse {
    pub id: i32,
    pub vehicle_id: i32,
    pub customer_id: i32,
    pub seller_id: i32,
    pub order_id: String,
    pub pickup_date: DateTime<Utc>,
    pub return_date: DateTime<Utc>,
    pub actual_pickup_at: Option<DateTime<Utc>>,
    pub actual_return_at: Option<DateTime<Utc>>,
    pub customer_name: String,
    pub customer_phone: String,
    pub customer_email: String,
    pub ktp_photo: Option<String>,
    pub total_days: i32,
    pub price_per_day: f64,
    pub total_price: f64,
    pub notes: Option<String>,
    pub status: String,
    pub cancel_reason: Option<String>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<RentalBooking> for RentalBookingResponse {
    fn from(booking: RentalBooking) -> Self {
        Self {
            id: booking.id,
            vehicle_id: booking.vehicle_id,
            customer_id: booking.customer_id,
            seller_id: booking.seller_id,
            order_id: booking.order_id,
            pickup_date: booking.pickup_date,
            return_date: booking.return_date,
            actual_pickup_at: booking.actual_pickup_at,
            actual_return_at: booking.actual_return_at,
            customer_name: booking.customer_name,
            customer_phone: booking.customer_phone,
            customer_email: booking.customer_email,
            ktp_photo: booking.ktp_photo,
            total_days: booking.total_days,
            price_per_day: booking.price_per_day,
            total_price: booking.total_price,
            notes: booking.notes,
            status: booking.status,
            cancel_reason: booking.cancel_reason,
            cancelled_at: booking.cancelled_at,
            created_at: booking.created_at,
            updated_at: booking.updated_at,
        }
    }
}
