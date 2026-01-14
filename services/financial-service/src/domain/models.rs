use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// Saldo seller berdasarkan seller_balance table
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SellerBalance {
    pub seller_id: i32,
    pub available_balance: f64,
    pub pending_balance: f64,
    pub total_earned: f64,
    pub updated_at: DateTime<Utc>,
}

// Response DTO untuk GET /api/seller/balance
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BalanceResponse {
    pub seller_id: i32,
    pub available_balance: f64,
    pub pending_balance: f64,
    pub total_earned: f64,
    pub updated_at: DateTime<Utc>,
}

impl From<SellerBalance> for BalanceResponse {
    fn from(balance: SellerBalance) -> Self {
        Self {
            seller_id: balance.seller_id,
            available_balance: balance.available_balance,
            pending_balance: balance.pending_balance,
            total_earned: balance.total_earned,
            updated_at: balance.updated_at,
        }
    }
}

// Transaction type enum 
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum TransactionType {
    RentalPayment,
    RentalRefund,
    SalePayment,
    SellerCredit,
    SellerWithdrawal,
    CommissionDeduction,
}

impl From<&str> for TransactionType {
    fn from(s: &str) -> Self {
        match s {
            "rental_payment" => TransactionType::RentalPayment,
            "rental_refund" => TransactionType::RentalRefund,
            "sale_payment" => TransactionType::SalePayment,
            "seller_credit" => TransactionType::SellerCredit,
            "seller_withdrawal" => TransactionType::SellerWithdrawal,
            "commission_deduction" => TransactionType::CommissionDeduction,
            _ => panic!("Invalid transaction type: {}", s),
        }
    }
}

impl TransactionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TransactionType::RentalPayment => "rental_payment",
            TransactionType::RentalRefund => "rental_refund",
            TransactionType::SalePayment => "sale_payment",
            TransactionType::SellerCredit => "seller_credit",
            TransactionType::SellerWithdrawal => "seller_withdrawal",
            TransactionType::CommissionDeduction => "commission_deduction",
        }
    }
}

// Implement Display untuk logging
impl std::fmt::Display for TransactionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// Transaction status enum
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum TransactionStatus {
    Pending,
    Completed,
    Failed,
    Reversed,
}

impl From<&str> for TransactionStatus {
    fn from(s: &str) -> Self {
        match s {
            "pending" => TransactionStatus::Pending,
            "completed" => TransactionStatus::Completed,
            "failed" => TransactionStatus::Failed,
            "reversed" => TransactionStatus::Reversed,
            _ => panic!("Invalid transaction status: {}", s),
        }
    }
}

impl TransactionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TransactionStatus::Pending => "pending",
            TransactionStatus::Completed => "completed",
            TransactionStatus::Failed => "failed",
            TransactionStatus::Reversed => "reversed",
        }
    }
}

// Implement Display untuk logging
impl std::fmt::Display for TransactionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// Transaction log dari transaction_logs table dengan type-safe enums
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TransactionLog {
    pub id: i32,
    pub transaction_type: TransactionType,
    pub amount: f64,
    pub commission_amount: Option<f64>,
    pub net_amount: Option<f64>,
    pub status: TransactionStatus,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

// Deserialize implementation untuk convert dari database strings
impl<'de> serde::Deserialize<'de> for TransactionLog {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct TempTransactionLog {
            pub id: i32,
            pub transaction_type: String,
            pub amount: f64,
            pub commission_amount: Option<f64>,
            pub net_amount: Option<f64>,
            pub status: String,
            pub notes: Option<String>,
            pub created_at: DateTime<Utc>,
        }

        let temp = TempTransactionLog::deserialize(deserializer)?;

        Ok(TransactionLog {
            id: temp.id,
            transaction_type: TransactionType::from(temp.transaction_type.as_str()),
            amount: temp.amount,
            commission_amount: temp.commission_amount,
            net_amount: temp.net_amount,
            status: TransactionStatus::from(temp.status.as_str()),
            notes: temp.notes,
            created_at: temp.created_at,
        })
    }
}

// Response DTO untuk GET /api/seller/transactions dengan pagination
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TransactionsResponse {
    pub transactions: Vec<TransactionLog>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

// Query parameters untuk transaction history
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct TransactionsQuery {
    pub transaction_type: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// Withdrawal status enum
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum WithdrawalStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

impl From<&str> for WithdrawalStatus {
    fn from(s: &str) -> Self {
        match s {
            "pending" => WithdrawalStatus::Pending,
            "processing" => WithdrawalStatus::Processing,
            "completed" => WithdrawalStatus::Completed,
            "failed" => WithdrawalStatus::Failed,
            _ => panic!("Invalid withdrawal status: {}", s),
        }
    }
}

impl WithdrawalStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            WithdrawalStatus::Pending => "pending",
            WithdrawalStatus::Processing => "processing",
            WithdrawalStatus::Completed => "completed",
            WithdrawalStatus::Failed => "failed",
        }
    }
}

// Implement Display untuk logging
impl std::fmt::Display for WithdrawalStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// Request DTO untuk POST /api/seller/withdrawals
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateWithdrawalRequest {
    pub amount: f64,
    pub bank_name: String,
    pub account_number: String,
    pub account_holder_name: String,
}

// Response DTO untuk POST /api/seller/withdrawals 
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct WithdrawalResponse {
    pub id: i32,
    pub seller_id: i32,
    pub amount: f64,
    pub bank_name: String,
    pub account_number: String,
    pub account_holder_name: String,
    pub status: WithdrawalStatus,
    pub requested_at: DateTime<Utc>,
}

// Withdrawal dari withdrawals table 
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Withdrawal {
    pub id: i32,
    pub seller_id: i32,
    pub amount: f64,
    pub bank_name: String,
    pub account_number: String,
    pub account_holder_name: String,
    pub status: WithdrawalStatus,
    pub requested_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

// Deserialize implementation untuk Withdrawal dari database strings
impl<'de> serde::Deserialize<'de> for Withdrawal {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct TempWithdrawal {
            pub id: i32,
            pub seller_id: i32,
            pub amount: f64,
            pub bank_name: String,
            pub account_number: String,
            pub account_holder_name: String,
            pub status: String,
            pub requested_at: DateTime<Utc>,
            pub processed_at: Option<DateTime<Utc>>,
            pub completed_at: Option<DateTime<Utc>>,
        }

        let temp = TempWithdrawal::deserialize(deserializer)?;

        Ok(Withdrawal {
            id: temp.id,
            seller_id: temp.seller_id,
            amount: temp.amount,
            bank_name: temp.bank_name,
            account_number: temp.account_number,
            account_holder_name: temp.account_holder_name,
            status: WithdrawalStatus::from(temp.status.as_str()),
            requested_at: temp.requested_at,
            processed_at: temp.processed_at,
            completed_at: temp.completed_at,
        })
    }
}

// Response DTO untuk GET /api/seller/withdrawals 
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct WithdrawalsListResponse {
    pub withdrawals: Vec<Withdrawal>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

// Query parameters untuk withdrawal list
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct WithdrawalsListQuery {
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}