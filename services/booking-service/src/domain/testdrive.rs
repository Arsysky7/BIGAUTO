use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::JsonValue;
use utoipa::ToSchema;

// Model utama TestDriveBooking dari database
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TestDriveBooking {
    pub id: i32,
    pub vehicle_id: i32,
    pub customer_id: i32,
    pub seller_id: i32,
    pub requested_date: DateTime<Utc>,
    pub requested_time: String,
    pub reschedule_slots: Option<JsonValue>,
    pub customer_name: String,
    pub customer_phone: String,
    pub customer_email: String,
    pub notes: Option<String>,
    pub status: String,
    pub timeout_at: Option<DateTime<Utc>>,
    pub cancel_reason: Option<String>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Enum untuk status test drive booking
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TestDriveStatus {
    MenungguKonfirmasi,
    SellerReschedule,
    Diterima,
    Selesai,
    Cancelled,
    Timeout,
}

impl TestDriveStatus {
    pub fn as_str(&self) -> &str {
        match self {
            TestDriveStatus::MenungguKonfirmasi => "menunggu_konfirmasi",
            TestDriveStatus::SellerReschedule => "seller_reschedule",
            TestDriveStatus::Diterima => "diterima",
            TestDriveStatus::Selesai => "selesai",
            TestDriveStatus::Cancelled => "cancelled",
            TestDriveStatus::Timeout => "timeout",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "menunggu_konfirmasi" => Some(TestDriveStatus::MenungguKonfirmasi),
            "seller_reschedule" => Some(TestDriveStatus::SellerReschedule),
            "diterima" => Some(TestDriveStatus::Diterima),
            "selesai" => Some(TestDriveStatus::Selesai),
            "cancelled" => Some(TestDriveStatus::Cancelled),
            "timeout" => Some(TestDriveStatus::Timeout),
            _ => None,
        }
    }
}

// Request untuk create test drive booking
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateTestDriveRequest {
    #[schema(example = 1)]
    pub vehicle_id: i32,
    #[schema(example = "2025-12-01T10:00:00Z")]
    pub requested_date: DateTime<Utc>,
    #[schema(example = "10:00")]
    pub requested_time: String,
    #[schema(example = "John Doe")]
    pub customer_name: String,
    #[schema(example = "081234567890")]
    pub customer_phone: String,
    #[schema(example = "john@example.com")]
    pub customer_email: String,
    #[schema(example = "Ingin test drive sebelum membeli")]
    pub notes: Option<String>,
}

// Request untuk seller reschedule test drive
#[derive(Debug, Deserialize, ToSchema)]
pub struct RescheduleTestDriveRequest {
    #[schema(example = json!([
        {"date": "2025-12-02T10:00:00Z", "time": "10:00"},
        {"date": "2025-12-03T14:00:00Z", "time": "14:00"}
    ]))]
    pub reschedule_slots: serde_json::Value,
}

// Request untuk customer pilih slot reschedule
#[derive(Debug, Deserialize, ToSchema)]
pub struct ChooseRescheduleSlotRequest {
    #[schema(example = 0)]
    pub slot_index: usize,
}

// Request untuk confirm test drive (seller)
#[derive(Debug, Deserialize, ToSchema)]
pub struct ConfirmTestDriveRequest {
    #[schema(example = "diterima")]
    pub status: String,
}

// Request untuk complete test drive (seller)
#[derive(Debug, Deserialize, ToSchema)]
pub struct CompleteTestDriveRequest {
    #[schema(example = "Test drive selesai, customer puas")]
    pub notes: Option<String>,
}

// Request untuk cancel test drive
#[derive(Debug, Deserialize, ToSchema)]
pub struct CancelTestDriveRequest {
    #[schema(example = "Ada keperluan mendadak")]
    pub cancel_reason: String,
}


// Response untuk test drive booking
#[derive(Debug, Serialize, ToSchema)]
pub struct TestDriveBookingResponse {
    pub id: i32,
    pub vehicle_id: i32,
    pub customer_id: i32,
    pub seller_id: i32,
    pub requested_date: DateTime<Utc>,
    pub requested_time: String,
    pub reschedule_slots: Option<JsonValue>,
    pub customer_name: String,
    pub customer_phone: String,
    pub customer_email: String,
    pub notes: Option<String>,
    pub status: String,
    pub timeout_at: Option<DateTime<Utc>>,
    pub cancel_reason: Option<String>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<TestDriveBooking> for TestDriveBookingResponse {
    fn from(booking: TestDriveBooking) -> Self {
        Self {
            id: booking.id,
            vehicle_id: booking.vehicle_id,
            customer_id: booking.customer_id,
            seller_id: booking.seller_id,
            requested_date: booking.requested_date,
            requested_time: booking.requested_time,
            reschedule_slots: booking.reschedule_slots,
            customer_name: booking.customer_name,
            customer_phone: booking.customer_phone,
            customer_email: booking.customer_email,
            notes: booking.notes,
            status: booking.status,
            timeout_at: booking.timeout_at,
            cancel_reason: booking.cancel_reason,
            cancelled_at: booking.cancelled_at,
            created_at: booking.created_at,
            updated_at: booking.updated_at,
        }
    }
}
