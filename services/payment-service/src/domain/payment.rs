use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// Model data payment transaction
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct Payment {
    pub id: i32,
    pub rental_booking_id: Option<i32>,
    pub sale_order_id: Option<i32>,
    pub order_id: String,

    // Midtrans transaction info
    pub transaction_id: Option<String>,
    pub va_number: Option<String>,
    pub bank: Option<String>,
    pub payment_type: Option<String>,

    // Amount & currency
    pub gross_amount: i64,

    // Payment status & type
    pub status: PaymentStatus,
    pub payment_for_type: PaymentType,

    // Refund info
    pub refund_amount: Option<i64>,
    pub refund_reason: Option<String>,

    // Timestamps
    pub paid_at: Option<DateTime<Utc>>,
    pub expired_at: Option<DateTime<Utc>>,
    pub refunded_at: Option<DateTime<Utc>>,

    // Document path
    pub receipt_pdf_path: Option<String>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Status payment
#[derive(Debug, Clone, Copy, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema, PartialEq)]
#[sqlx(type_name = "varchar", rename_all = "lowercase")]
pub enum PaymentStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "expired")]
    Expired,
    #[serde(rename = "refunded")]
    Refunded,
}

impl std::fmt::Display for PaymentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PaymentStatus::Pending => write!(f, "pending"),
            PaymentStatus::Success => write!(f, "success"),
            PaymentStatus::Failed => write!(f, "failed"),
            PaymentStatus::Expired => write!(f, "expired"),
            PaymentStatus::Refunded => write!(f, "refunded"),
        }
    }
}

// Tipe transaksi
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema)]
#[sqlx(type_name = "varchar", rename_all = "lowercase")]
pub enum PaymentType {
    #[serde(rename = "rental")]
    Rental,
    #[serde(rename = "sale")]
    Sale,
}

impl std::fmt::Display for PaymentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PaymentType::Rental => write!(f, "rental"),
            PaymentType::Sale => write!(f, "sale"),
        }
    }
}

// Request buat payment baru
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CreatePaymentRequest {
    pub payment_for_type: PaymentType,
    pub rental_booking_id: Option<i32>,
    pub sale_order_id: Option<i32>,
    pub gross_amount: i64,
    pub payment_method: String,
    pub customer_details: CustomerDetails,
    pub item_details: Vec<ItemDetails>,
}

// Data customer untuk Midtrans
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema, Clone)]
pub struct CustomerDetails {
    pub first_name: String,
    pub last_name: Option<String>,
    pub email: String,
    pub phone: String,
}

// Detail item yang dibayar
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema, Clone)]
pub struct ItemDetails {
    pub id: String,
    pub name: String,
    pub price: i64,
    pub quantity: i32,
}

// Request charge ke Midtrans
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MidtransChargeRequest {
    pub payment_type: String,
    pub transaction_details: TransactionDetails,
    pub customer_details: CustomerDetails,
    pub item_details: Vec<ItemDetails>,
    pub bank_transfer: Option<BankTransfer>,
}

// Detail transaction untuk Midtrans
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TransactionDetails {
    pub order_id: String,
    pub gross_amount: i64,
}

// Info bank transfer
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BankTransfer {
    pub bank: String,
    pub va_number: String,
}

// Response dari Midtrans charge
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MidtransChargeResponse {
    pub status_code: String,
    pub status_message: String,
    pub transaction_id: String,
    pub order_id: String,
    pub gross_amount: String,
    pub payment_type: String,
    pub transaction_status: String,
    pub transaction_time: String,
    pub va_numbers: Option<Vec<VaNumber>>,
    pub expiry_time: Option<String>,
    pub bank_transfer: Option<BankTransfer>,
}

// Info VA number
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct VaNumber {
    pub bank: String,
    pub va_number: String,
}

// Payload webhook dari Midtrans
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MidtransWebhookPayload {
    pub transaction_status: String,
    pub transaction_id: String,
    pub status_code: String,
    pub order_id: String,
    pub gross_amount: String,
    pub payment_type: String,
    pub transaction_time: String,
    pub fraud_status: Option<String>,
    pub va_numbers: Option<Vec<VaNumber>>,
}

// Request refund payment
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RefundRequest {
    pub order_id: String,
    pub refund_amount: i64,
    pub reason: String,
}

// Business logic methods
impl Payment {
    /// Cek apakah payment sudah expired
    pub fn is_expired(&self) -> bool {
        self.expired_at
            .map(|expired| Utc::now() > expired)
            .unwrap_or(false)
    }

    /// Cek apakah payment bisa direfund
    pub fn can_be_refunded(&self) -> bool {
        matches!(self.status, PaymentStatus::Success) && !self.is_expired()
    }

    /// Generate order ID unik
    pub fn generate_order_id(payment_type: PaymentType) -> String {
        let prefix = match payment_type {
            PaymentType::Rental => "RNT",
            PaymentType::Sale => "SAL",
        };

        let date = Utc::now().format("%Y%m%d");
        let random: u32 = rand::random();
        format!("{}-{}-{:05}", prefix, date, random % 100000)
    }

    /// Generate expiry time (24 jam untuk rental, 48 jam untuk sale)
    pub fn generate_expiry_time(payment_type: PaymentType) -> DateTime<Utc> {
        let hours = match payment_type {
            PaymentType::Rental => 24,
            PaymentType::Sale => 48,
        };
        Utc::now() + chrono::Duration::hours(hours)
    }
}
// Webhook response untuk Midtrans callback
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct WebhookResponse {
    pub success: bool,
    pub message: String,
    pub order_id: String,
    pub status: PaymentStatus,
    pub transaction_id: String,
}

// Payment receipt untuk payment yang sukses
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PaymentReceipt {
    pub receipt_id: String,
    pub order_id: String,
    pub transaction_id: String,
    pub payment_type: PaymentType,
    pub gross_amount: i64,
    pub payment_method: String,
    pub customer_name: String,
    pub customer_email: String,
    pub paid_at: DateTime<Utc>,
    pub receipt_url: String,
}

impl PaymentReceipt {
    pub fn from_payment(payment: &Payment) -> Self {
        Self {
            receipt_id: format!("RCP-{}", payment.order_id),
            order_id: payment.order_id.clone(),
            transaction_id: payment.transaction_id.clone().unwrap_or_default(),
            payment_type: payment.payment_for_type.clone(),
            gross_amount: payment.gross_amount,
            payment_method: "Virtual Account".to_string(), 
            customer_name: "Customer".to_string(), 
            customer_email: "customer@example.com".to_string(), 
            paid_at: payment.paid_at.unwrap_or_else(Utc::now),
            receipt_url: format!("/receipts/{}", payment.order_id),
        }
    }
}