use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// Model utama SaleOrder dari database
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SaleOrder {
    pub id: i32,
    pub vehicle_id: i32,
    pub buyer_id: i32,
    pub seller_id: i32,
    pub testdrive_booking_id: Option<i32>,
    pub order_id: String,
    pub asking_price: f64,
    pub offer_price: Option<f64>,
    pub counter_offer_price: Option<f64>,
    pub final_price: f64,
    pub buyer_name: String,
    pub buyer_phone: String,
    pub buyer_email: String,
    pub buyer_address: Option<String>,
    pub buyer_ktp_photo: Option<String>,
    pub status: String,
    pub bpkb_transferred: bool,
    pub stnk_transferred: bool,
    pub faktur_transferred: bool,
    pub pajak_transferred: bool,
    pub created_at: DateTime<Utc>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub paid_at: Option<DateTime<Utc>>,
    pub document_transfer_started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
    pub cancel_reason: Option<String>,
    pub reject_reason: Option<String>,
    pub rejected_at: Option<DateTime<Utc>>,
    pub buyer_notes: Option<String>,
    pub seller_notes: Option<String>,
}

// Enum untuk status sale order
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SaleStatus {
    PendingConfirmation,
    PendingPayment,
    Paid,
    DocumentProcessing,
    Completed,
    Cancelled,
    Rejected,
}

impl SaleStatus {
    pub fn as_str(&self) -> &str {
        match self {
            SaleStatus::PendingConfirmation => "pending_confirmation",
            SaleStatus::PendingPayment => "pending_payment",
            SaleStatus::Paid => "paid",
            SaleStatus::DocumentProcessing => "document_processing",
            SaleStatus::Completed => "completed",
            SaleStatus::Cancelled => "cancelled",
            SaleStatus::Rejected => "rejected",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending_confirmation" => Some(SaleStatus::PendingConfirmation),
            "pending_payment" => Some(SaleStatus::PendingPayment),
            "paid" => Some(SaleStatus::Paid),
            "document_processing" => Some(SaleStatus::DocumentProcessing),
            "completed" => Some(SaleStatus::Completed),
            "cancelled" => Some(SaleStatus::Cancelled),
            "rejected" => Some(SaleStatus::Rejected),
            _ => None,
        }
    }
}

// Request untuk create sale order (buyer)
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateSaleOrderRequest {
    #[schema(example = 1)]
    pub vehicle_id: i32,
    #[schema(example = 1)]
    pub testdrive_booking_id: Option<i32>,
    #[schema(example = 250000000.0)]
    pub offer_price: Option<f64>,
    #[schema(example = "John Doe")]
    pub buyer_name: String,
    #[schema(example = "081234567890")]
    pub buyer_phone: String,
    #[schema(example = "john@example.com")]
    pub buyer_email: String,
    #[schema(example = "Jl. Sudirman No. 123, Jakarta")]
    pub buyer_address: Option<String>,
    #[schema(example = "Saya tertarik dengan mobil ini")]
    pub buyer_notes: Option<String>,
}


// Request untuk reject sale order
#[derive(Debug, Deserialize, ToSchema)]
pub struct RejectSaleOrderRequest {
    #[schema(example = "Mobil sudah tidak tersedia")]
    pub reject_reason: String,
}


// Request untuk buyer accept counter offer
#[derive(Debug, Deserialize, ToSchema)]
pub struct AcceptCounterOfferRequest {
    #[schema(example = true)]
    pub accept: bool,
}

// Request untuk buyer accept sale order (seller confirmation)
#[derive(Debug, Deserialize, ToSchema)]
pub struct AcceptSaleOrderRequest {
    #[schema(example = true)]
    pub accept: bool,
    #[schema(example = "Saya setuju dengan harga ini")]
    pub notes: Option<String>,
}

// Request untuk seller counter offer
#[derive(Debug, Deserialize, ToSchema)]
pub struct CounterOfferRequest {
    #[schema(example = 245000000.0)]
    pub counter_price: f64,
    #[schema(example = "Harga ini masih terlalu tinggi, saya tawar 245 juta")]
    pub reason: Option<String>,
}

// Request untuk cancel sale order
#[derive(Debug, Deserialize, ToSchema)]
pub struct CancelRequest {
    #[schema(example = "Berubah pikiran")]
    pub reason: String,
}


// Request untuk upload KTP (buyer)
#[derive(Debug, Deserialize, ToSchema)]
pub struct UploadKtpRequest {
    #[schema(example = "https://cloudinary.com/ktp.jpg")]
    pub ktp_photo: String,
}

// Request untuk start document transfer (seller)
#[derive(Debug, Deserialize, ToSchema)]
pub struct StartDocumentTransferRequest {
    #[schema(example = "Proses serah terima dokumen dimulai")]
    pub notes: Option<String>,
}

// Request untuk update document transfer status (seller)
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateDocumentStatusRequest {
    #[schema(example = true)]
    pub bpkb_transferred: Option<bool>,
    #[schema(example = true)]
    pub stnk_transferred: Option<bool>,
    #[schema(example = true)]
    pub faktur_transferred: Option<bool>,
    #[schema(example = true)]
    pub pajak_transferred: Option<bool>,
}


// Query parameters untuk sale order filtering
#[derive(Debug, Deserialize, ToSchema)]
pub struct SaleOrderQueryParams {
    pub status: Option<String>,
    pub page: Option<i32>,
    pub limit: Option<i32>,
}

// Response untuk sale order
#[derive(Debug, Serialize, ToSchema)]
pub struct SaleOrderResponse {
    pub id: i32,
    pub vehicle_id: i32,
    pub buyer_id: i32,
    pub seller_id: i32,
    pub testdrive_booking_id: Option<i32>,
    pub order_id: String,
    pub asking_price: f64,
    pub offer_price: Option<f64>,
    pub counter_offer_price: Option<f64>,
    pub final_price: f64,
    pub buyer_name: String,
    pub buyer_phone: String,
    pub buyer_email: String,
    pub buyer_address: Option<String>,
    pub buyer_ktp_photo: Option<String>,
    pub status: String,
    pub bpkb_transferred: bool,
    pub stnk_transferred: bool,
    pub faktur_transferred: bool,
    pub pajak_transferred: bool,
    pub created_at: DateTime<Utc>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub paid_at: Option<DateTime<Utc>>,
    pub document_transfer_started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
    pub cancel_reason: Option<String>,
    pub reject_reason: Option<String>,
    pub rejected_at: Option<DateTime<Utc>>,
    pub buyer_notes: Option<String>,
    pub seller_notes: Option<String>,
}

impl From<SaleOrder> for SaleOrderResponse {
    fn from(order: SaleOrder) -> Self {
        Self {
            id: order.id,
            vehicle_id: order.vehicle_id,
            buyer_id: order.buyer_id,
            seller_id: order.seller_id,
            testdrive_booking_id: order.testdrive_booking_id,
            order_id: order.order_id,
            asking_price: order.asking_price,
            offer_price: order.offer_price,
            counter_offer_price: order.counter_offer_price,
            final_price: order.final_price,
            buyer_name: order.buyer_name,
            buyer_phone: order.buyer_phone,
            buyer_email: order.buyer_email,
            buyer_address: order.buyer_address,
            buyer_ktp_photo: order.buyer_ktp_photo,
            status: order.status,
            bpkb_transferred: order.bpkb_transferred,
            stnk_transferred: order.stnk_transferred,
            faktur_transferred: order.faktur_transferred,
            pajak_transferred: order.pajak_transferred,
            created_at: order.created_at,
            confirmed_at: order.confirmed_at,
            paid_at: order.paid_at,
            document_transfer_started_at: order.document_transfer_started_at,
            completed_at: order.completed_at,
            cancelled_at: order.cancelled_at,
            updated_at: order.updated_at,
            cancel_reason: order.cancel_reason,
            reject_reason: order.reject_reason,
            rejected_at: order.rejected_at,
            buyer_notes: order.buyer_notes,
            seller_notes: order.seller_notes,
        }
    }
}
