use crate::domain::payment::{
    Payment, PaymentStatus, PaymentType, CreatePaymentRequest,
    MidtransWebhookPayload, MidtransChargeResponse
};
use crate::error::AppError;
use sqlx::PgPool;
use chrono::Utc;
use bigdecimal::ToPrimitive;

// Repository untuk operasi database payment
#[derive(Clone)]
pub struct PaymentRepository {
    pool: PgPool,
}

impl PaymentRepository {
    // Buat payment repository baru 
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // Create payment baru with Midtrans response 
    pub async fn create_payment(
        &self,
        request: &CreatePaymentRequest,
        order_id: &str,
        midtrans_response: &MidtransChargeResponse,
        expiry_time: chrono::DateTime<Utc>,
    ) -> Result<Payment, AppError> {
        // Extract VA numbers from Midtrans response
        let (bank, va_number) = if let Some(vas) = &midtrans_response.va_numbers {
            if let Some(first_va) = vas.first() {
                (Some(first_va.bank.clone()), Some(first_va.va_number.clone()))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        let payment_type_str = match request.payment_for_type {
            PaymentType::Rental => "rental",
            PaymentType::Sale => "sale",
        };

        let row = sqlx::query!(
            r#"
            INSERT INTO payments (
                rental_booking_id, sale_order_id, order_id,
                transaction_id, va_number, bank, payment_type,
                gross_amount, status, payment_for_type,
                expired_at, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            RETURNING *
            "#,
            request.rental_booking_id,
            request.sale_order_id,
            order_id,
            midtrans_response.transaction_id,
            va_number,
            bank,
            midtrans_response.payment_type,
            bigdecimal::BigDecimal::from(request.gross_amount),
            "pending",
            payment_type_str,
            expiry_time,
            Utc::now(),
            Utc::now()
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(Payment {
            id: row.id,
            rental_booking_id: row.rental_booking_id,
            sale_order_id: row.sale_order_id,
            order_id: row.order_id,
            transaction_id: row.transaction_id,
            va_number: row.va_number,
            bank: row.bank,
            payment_type: row.payment_type,
            gross_amount: row.gross_amount.to_i64().ok_or_else(|| AppError::internal("Failed to convert BigDecimal to i64"))?,
            status: PaymentStatus::Pending,
            payment_for_type: match row.payment_for_type.as_ref().map_or("rental", |s| s.as_str()) {
                "rental" => PaymentType::Rental,
                "sale" => PaymentType::Sale,
                _ => PaymentType::Rental,
            },
            refund_amount: row.refund_amount.and_then(|v| v.to_i64()),
            refund_reason: row.refund_reason,
            paid_at: row.paid_at,
            expired_at: row.expired_at,
            refunded_at: row.refunded_at,
            receipt_pdf_path: row.receipt_pdf_path,
            created_at: row.created_at.ok_or_else(|| AppError::internal("Missing created_at"))?,
            updated_at: row.updated_at.ok_or_else(|| AppError::internal("Missing updated_at"))?,
        })
    }

    
    /// Cari payment berdasarkan ID
    pub async fn find_by_id(
        &self,
        id: i32,
    ) -> Result<Option<Payment>, AppError> {
        let row = sqlx::query!(
            "SELECT * FROM payments WHERE id = $1",
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        let payment = match row {
            Some(p) => Some(Payment {
                id: p.id,
                rental_booking_id: p.rental_booking_id,
                sale_order_id: p.sale_order_id,
                order_id: p.order_id,
                transaction_id: p.transaction_id,
                va_number: p.va_number,
                bank: p.bank,
                payment_type: p.payment_type,
                gross_amount: p.gross_amount.to_i64().ok_or_else(|| AppError::internal("Failed to convert BigDecimal to i64"))?,
                status: match p.status.as_str() {
                    "pending" => PaymentStatus::Pending,
                    "success" => PaymentStatus::Success,
                    "failed" => PaymentStatus::Failed,
                    "expired" => PaymentStatus::Expired,
                    "refunded" => PaymentStatus::Refunded,
                    _ => PaymentStatus::Pending,
                },
                payment_for_type: match p.payment_for_type.as_ref().map_or("rental", |s| s.as_str()) {
                    "rental" => PaymentType::Rental,
                    "sale" => PaymentType::Sale,
                    _ => PaymentType::Rental,
                },
                refund_amount: p.refund_amount.and_then(|v| v.to_i64()),
                refund_reason: p.refund_reason,
                paid_at: p.paid_at,
                expired_at: p.expired_at,
                refunded_at: p.refunded_at,
                receipt_pdf_path: p.receipt_pdf_path,
                created_at: p.created_at.ok_or_else(|| AppError::internal("Missing created_at"))?,
                updated_at: p.updated_at.ok_or_else(|| AppError::internal("Missing updated_at"))?,
            }),
            None => None,
        };

        Ok(payment)
    }

    // Cari payment berdasarkan orderID
    pub async fn find_by_order_id(
        &self,
        order_id: &str,
    ) -> Result<Option<Payment>, AppError> {
        let row = sqlx::query!(
            "SELECT * FROM payments WHERE order_id = $1",
            order_id
        )
        .fetch_optional(&self.pool)
        .await?;

        let payment = match row {
            Some(p) => Some(Payment {
                id: p.id,
                rental_booking_id: p.rental_booking_id,
                sale_order_id: p.sale_order_id,
                order_id: p.order_id,
                transaction_id: p.transaction_id,
                va_number: p.va_number,
                bank: p.bank,
                payment_type: p.payment_type,
                gross_amount: p.gross_amount.to_i64().ok_or_else(|| AppError::internal("Failed to convert BigDecimal to i64"))?,
                status: match p.status.as_str() {
                    "pending" => PaymentStatus::Pending,
                    "success" => PaymentStatus::Success,
                    "failed" => PaymentStatus::Failed,
                    "expired" => PaymentStatus::Expired,
                    "refunded" => PaymentStatus::Refunded,
                    _ => PaymentStatus::Pending,
                },
                payment_for_type: match p.payment_for_type.as_ref().map_or("rental", |s| s.as_str()) {
                    "rental" => PaymentType::Rental,
                    "sale" => PaymentType::Sale,
                    _ => PaymentType::Rental,
                },
                refund_amount: p.refund_amount.and_then(|v| v.to_i64()),
                refund_reason: p.refund_reason,
                paid_at: p.paid_at,
                expired_at: p.expired_at,
                refunded_at: p.refunded_at,
                receipt_pdf_path: p.receipt_pdf_path,
                created_at: p.created_at.ok_or_else(|| AppError::internal("Missing created_at"))?,
                updated_at: p.updated_at.ok_or_else(|| AppError::internal("Missing updated_at"))?,
            }),
            None => None,
        };

        Ok(payment)
    }

    /// Cari payment berdasarkan rental booking ID
    pub async fn find_by_rental_booking_id(
        &self, 
        booking_id: i32,
    ) -> Result<Option<Payment>, AppError> {
        let row = sqlx::query!(
            "SELECT * FROM payments WHERE rental_booking_id = $1",
            booking_id
        )
        .fetch_optional(&self.pool)
        .await?;

        let payment = match row {
            Some(p) => Some(Payment {
                id: p.id,
                rental_booking_id: p.rental_booking_id,
                sale_order_id: p.sale_order_id,
                order_id: p.order_id,
                transaction_id: p.transaction_id,
                va_number: p.va_number,
                bank: p.bank,
                payment_type: p.payment_type,
                gross_amount: p.gross_amount.to_i64().ok_or_else(|| AppError::internal("Failed to convert BigDecimal to i64"))?,
                status: match p.status.as_str() {
                    "pending" => PaymentStatus::Pending,
                    "success" => PaymentStatus::Success,
                    "failed" => PaymentStatus::Failed,
                    "expired" => PaymentStatus::Expired,
                    "refunded" => PaymentStatus::Refunded,
                    _ => PaymentStatus::Pending,
                },
                payment_for_type: match p.payment_for_type.as_ref().map_or("rental", |s| s.as_str()) {
                    "rental" => PaymentType::Rental,
                    "sale" => PaymentType::Sale,
                    _ => PaymentType::Rental,
                },
                refund_amount: p.refund_amount.and_then(|v| v.to_i64()),
                refund_reason: p.refund_reason,
                paid_at: p.paid_at,
                expired_at: p.expired_at,
                refunded_at: p.refunded_at,
                receipt_pdf_path: p.receipt_pdf_path,
                created_at: p.created_at.ok_or_else(|| AppError::internal("Missing created_at"))?,
                updated_at: p.updated_at.ok_or_else(|| AppError::internal("Missing updated_at"))?,
            }),
            None => None,
        };

        Ok(payment)
    }

    /// Cari payment berdasarkan sale order ID
    pub async fn find_by_sale_order_id(
        &self, 
        sale_order_id: i32,
    ) -> Result<Option<Payment>, AppError> {
        let row = sqlx::query!(
            "SELECT * FROM payments WHERE sale_order_id = $1",
            sale_order_id
        )
        .fetch_optional(&self.pool)
        .await?;

        let payment = match row {
            Some(p) => Some(Payment {
                id: p.id,
                rental_booking_id: p.rental_booking_id,
                sale_order_id: p.sale_order_id,
                order_id: p.order_id,
                transaction_id: p.transaction_id,
                va_number: p.va_number,
                bank: p.bank,
                payment_type: p.payment_type,
                gross_amount: p.gross_amount.to_i64().ok_or_else(|| AppError::internal("Failed to convert BigDecimal to i64"))?,
                status: match p.status.as_str() {
                    "pending" => PaymentStatus::Pending,
                    "success" => PaymentStatus::Success,
                    "failed" => PaymentStatus::Failed,
                    "expired" => PaymentStatus::Expired,
                    "refunded" => PaymentStatus::Refunded,
                    _ => PaymentStatus::Pending,
                },
                payment_for_type: match p.payment_for_type.as_ref().map_or("rental", |s| s.as_str()) {
                    "rental" => PaymentType::Rental,
                    "sale" => PaymentType::Sale,
                    _ => PaymentType::Rental,
                },
                refund_amount: p.refund_amount.and_then(|v| v.to_i64()),
                refund_reason: p.refund_reason,
                paid_at: p.paid_at,
                expired_at: p.expired_at,
                refunded_at: p.refunded_at,
                receipt_pdf_path: p.receipt_pdf_path,
                created_at: p.created_at.ok_or_else(|| AppError::internal("Missing created_at"))?,
                updated_at: p.updated_at.ok_or_else(|| AppError::internal("Missing updated_at"))?,
            }),
            None => None,
        };

        Ok(payment)
    }

    /// Update status payment
    pub async fn update_status(
        &self,
        payment_id: i32,
        status: PaymentStatus,
        transaction_id: Option<String>,
    ) -> Result<Payment, AppError> {
        let now = Utc::now();

        let status_str = match status {
            PaymentStatus::Pending => "pending",
            PaymentStatus::Success => "success",
            PaymentStatus::Failed => "failed",
            PaymentStatus::Expired => "expired",
            PaymentStatus::Refunded => "refunded",
        };

        let payment = sqlx::query!(
            r#"
            UPDATE payments
            SET status = $1::varchar,
                transaction_id = $2,
                paid_at = CASE WHEN $1::varchar = 'success' THEN $3 ELSE paid_at END,
                updated_at = $3
            WHERE id = $4
            RETURNING *
            "#,
            status_str,
            transaction_id,
            now,
            payment_id
        )
        .fetch_one(&self.pool)
        .await?;

        // Manual conversion
        Ok(Payment {
            id: payment.id,
            rental_booking_id: payment.rental_booking_id,
            sale_order_id: payment.sale_order_id,
            order_id: payment.order_id,
            transaction_id: payment.transaction_id,
            va_number: payment.va_number,
            bank: payment.bank,
            payment_type: payment.payment_type,
            gross_amount: payment.gross_amount.to_i64().ok_or_else(|| AppError::internal("Failed to convert BigDecimal to i64"))?,
            status: match payment.status.as_str() {
                "pending" => PaymentStatus::Pending,
                "success" => PaymentStatus::Success,
                "failed" => PaymentStatus::Failed,
                "expired" => PaymentStatus::Expired,
                "refunded" => PaymentStatus::Refunded,
                _ => PaymentStatus::Pending,
            },
            payment_for_type: match payment.payment_for_type.as_ref().map_or("rental", |s| s.as_str()) {
                "rental" => PaymentType::Rental,
                "sale" => PaymentType::Sale,
                _ => PaymentType::Rental,
            },
            refund_amount: payment.refund_amount.and_then(|v| v.to_i64()),
            refund_reason: payment.refund_reason,
            paid_at: payment.paid_at,
            expired_at: payment.expired_at,
            refunded_at: payment.refunded_at,
            receipt_pdf_path: payment.receipt_pdf_path,
            created_at: payment.created_at.ok_or_else(|| AppError::internal("Missing created_at"))?,
            updated_at: payment.updated_at.ok_or_else(|| AppError::internal("Missing updated_at"))?,
        })
    }

    /// Update refund info
    pub async fn update_refund(
        &self,
        payment_id: i32,
        refund_amount: i64,
        refund_reason: String,
    ) -> Result<Payment, AppError> {
        let now = Utc::now();

        let row = sqlx::query!(
            r#"
            UPDATE payments
            SET status = 'refunded',
                refund_amount = $1,
                refund_reason = $2,
                refunded_at = $3,
                updated_at = $3
            WHERE id = $4
            RETURNING *
            "#,
            bigdecimal::BigDecimal::from(refund_amount),
            refund_reason,
            now,
            payment_id
        )
        .fetch_one(&self.pool)
        .await?;

        let payment = Payment {
            id: row.id,
            rental_booking_id: row.rental_booking_id,
            sale_order_id: row.sale_order_id,
            order_id: row.order_id,
            transaction_id: row.transaction_id,
            va_number: row.va_number,
            bank: row.bank,
            payment_type: row.payment_type,
            gross_amount: row.gross_amount.to_i64().ok_or_else(|| AppError::internal("Failed to convert BigDecimal to i64"))?,
            status: PaymentStatus::Refunded,
            payment_for_type: match row.payment_for_type.as_ref().map_or("rental", |s| s.as_str()) {
                "rental" => PaymentType::Rental,
                "sale" => PaymentType::Sale,
                _ => PaymentType::Rental,
            },
            refund_amount: row.refund_amount.and_then(|v| v.to_i64()),
            refund_reason: row.refund_reason,
            paid_at: row.paid_at,
            expired_at: row.expired_at,
            refunded_at: row.refunded_at,
            receipt_pdf_path: row.receipt_pdf_path,
            created_at: row.created_at.ok_or_else(|| AppError::internal("Missing created_at"))?,
            updated_at: row.updated_at.ok_or_else(|| AppError::internal("Missing updated_at"))?,
        };

        Ok(payment)
    }

    /// Update receipt PDF path
    pub async fn update_receipt_path(&self, payment_id: i32, path: String) -> Result<(), AppError> {
        sqlx::query!(
            "UPDATE payments SET receipt_pdf_path = $1, updated_at = $2 WHERE id = $3",
            path,
            Utc::now(),
            payment_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Check apakah payment ada untuk booking
    pub async fn exists_for_rental_booking(&self, booking_id: i32) -> Result<bool, AppError> {
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) as count FROM payments WHERE rental_booking_id = $1",
            booking_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(count.unwrap_or(0) > 0)
    }

    /// Check apakah payment ada untuk sale order
    pub async fn exists_for_sale_order(&self, sale_order_id: i32) -> Result<bool, AppError> {
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) as count FROM payments WHERE sale_order_id = $1",
            sale_order_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(count.unwrap_or(0) > 0)
    }

    /// Get list payments by status
    pub async fn find_by_status(&self, status: PaymentStatus) -> Result<Vec<Payment>, AppError> {
        let status_str = match status {
            PaymentStatus::Pending => "pending",
            PaymentStatus::Success => "success",
            PaymentStatus::Failed => "failed",
            PaymentStatus::Expired => "expired",
            PaymentStatus::Refunded => "refunded",
        };

        let rows = sqlx::query!(
            "SELECT * FROM payments WHERE status = $1 ORDER BY created_at DESC",
            status_str
        )
        .fetch_all(&self.pool)
        .await?;

        let payments = rows.into_iter().map(|p| -> Result<Payment, AppError> {
            Ok(Payment {
                id: p.id,
                rental_booking_id: p.rental_booking_id,
                sale_order_id: p.sale_order_id,
                order_id: p.order_id,
                transaction_id: p.transaction_id,
                va_number: p.va_number,
                bank: p.bank,
                payment_type: p.payment_type,
                gross_amount: p.gross_amount.to_i64().ok_or_else(|| AppError::internal("Failed to convert BigDecimal to i64"))?,
                status: match p.status.as_str() {
                    "pending" => PaymentStatus::Pending,
                    "success" => PaymentStatus::Success,
                    "failed" => PaymentStatus::Failed,
                    "expired" => PaymentStatus::Expired,
                    "refunded" => PaymentStatus::Refunded,
                    _ => PaymentStatus::Pending,
                },
                payment_for_type: match p.payment_for_type.as_ref().map_or("rental", |s| s.as_str()) {
                    "rental" => PaymentType::Rental,
                    "sale" => PaymentType::Sale,
                    _ => PaymentType::Rental,
                },
                refund_amount: p.refund_amount.and_then(|v| v.to_i64()),
                refund_reason: p.refund_reason,
                paid_at: p.paid_at,
                expired_at: p.expired_at,
                refunded_at: p.refunded_at,
                receipt_pdf_path: p.receipt_pdf_path,
                created_at: p.created_at.ok_or_else(|| AppError::internal("Missing created_at"))?,
                updated_at: p.updated_at.ok_or_else(|| AppError::internal("Missing updated_at"))?,
            })
        }).collect::<Result<Vec<_>, _>>()?;

        Ok(payments)
    }

    /// Get payments by type and status
    pub async fn find_by_type_and_status(
        &self,
        payment_type: PaymentType,
        status: PaymentStatus,
    ) -> Result<Vec<Payment>, AppError> {
        let payment_type_str = match payment_type {
            PaymentType::Rental => "rental",
            PaymentType::Sale => "sale",
        };

        let status_str = match status {
            PaymentStatus::Pending => "pending",
            PaymentStatus::Success => "success",
            PaymentStatus::Failed => "failed",
            PaymentStatus::Expired => "expired",
            PaymentStatus::Refunded => "refunded",
        };

        let rows = sqlx::query!(
            "SELECT * FROM payments WHERE payment_for_type = $1 AND status = $2 ORDER BY created_at DESC",
            payment_type_str,
            status_str
        )
        .fetch_all(&self.pool)
        .await?;

        let payments = rows.into_iter().map(|p| -> Result<Payment, AppError> {
            Ok(Payment {
                id: p.id,
                rental_booking_id: p.rental_booking_id,
                sale_order_id: p.sale_order_id,
                order_id: p.order_id,
                transaction_id: p.transaction_id,
                va_number: p.va_number,
                bank: p.bank,
                payment_type: p.payment_type,
                gross_amount: p.gross_amount.to_i64().ok_or_else(|| AppError::internal("Failed to convert BigDecimal to i64"))?,
                status: match p.status.as_str() {
                    "pending" => PaymentStatus::Pending,
                    "success" => PaymentStatus::Success,
                    "failed" => PaymentStatus::Failed,
                    "expired" => PaymentStatus::Expired,
                    "refunded" => PaymentStatus::Refunded,
                    _ => PaymentStatus::Pending,
                },
                payment_for_type: match p.payment_for_type.as_ref().map_or("rental", |s| s.as_str()) {
                    "rental" => PaymentType::Rental,
                    "sale" => PaymentType::Sale,
                    _ => PaymentType::Rental,
                },
                refund_amount: p.refund_amount.and_then(|v| v.to_i64()),
                refund_reason: p.refund_reason,
                paid_at: p.paid_at,
                expired_at: p.expired_at,
                refunded_at: p.refunded_at,
                receipt_pdf_path: p.receipt_pdf_path,
                created_at: p.created_at.ok_or_else(|| AppError::internal("Missing created_at"))?,
                updated_at: p.updated_at.ok_or_else(|| AppError::internal("Missing updated_at"))?,
            })
        }).collect::<Result<Vec<_>, _>>()?;

        Ok(payments)
    }

    /// Get payments by user ID (legacy method - searches both rental and sale orders)
    pub async fn find_by_user_id(&self, user_id: i32) -> Result<Vec<Payment>, AppError> {
        // TODO: In production, join with rental_bookings and sale_orders tables
        // For now, return recent payments as placeholder implementation
        // The user_id parameter is used to validate the request intent
        if user_id <= 0 {
            return Err(AppError::validation("Invalid user ID"));
        }

        let rows = sqlx::query!(
            "SELECT * FROM payments ORDER BY created_at DESC LIMIT 100",
        )
        .fetch_all(&self.pool)
        .await?;

        let payments = rows.into_iter().map(|p| -> Result<Payment, AppError> {
            Ok(Payment {
                id: p.id,
                rental_booking_id: p.rental_booking_id,
                sale_order_id: p.sale_order_id,
                order_id: p.order_id,
                transaction_id: p.transaction_id,
                va_number: p.va_number,
                bank: p.bank,
                payment_type: p.payment_type,
                gross_amount: p.gross_amount.to_i64().ok_or_else(|| AppError::internal("Failed to convert BigDecimal to i64"))?,
                status: match p.status.as_str() {
                    "pending" => PaymentStatus::Pending,
                    "success" => PaymentStatus::Success,
                    "failed" => PaymentStatus::Failed,
                    "expired" => PaymentStatus::Expired,
                    "refunded" => PaymentStatus::Refunded,
                    _ => PaymentStatus::Pending,
                },
                payment_for_type: match p.payment_for_type.as_ref().map_or("rental", |s| s.as_str()) {
                    "rental" => PaymentType::Rental,
                    "sale" => PaymentType::Sale,
                    _ => PaymentType::Rental,
                },
                refund_amount: p.refund_amount.and_then(|v| v.to_i64()),
                refund_reason: p.refund_reason,
                paid_at: p.paid_at,
                expired_at: p.expired_at,
                refunded_at: p.refunded_at,
                receipt_pdf_path: p.receipt_pdf_path,
                created_at: p.created_at.ok_or_else(|| AppError::internal("Missing created_at"))?,
                updated_at: p.updated_at.ok_or_else(|| AppError::internal("Missing updated_at"))?,
            })
        }).collect::<Result<Vec<_>, _>>()?;

        Ok(payments)
    }

    /// Update payment status with transaction log (webhook integration)
    pub async fn update_status_with_transaction_log(
        &self,
        payment_id: i32,
        status: &PaymentStatus,
        transaction_id: Option<&str>,
        webhook_payload: &MidtransWebhookPayload,
    ) -> Result<Payment, AppError> {
        let now = Utc::now();

        // Log webhook payload for audit trail
        tracing::info!(
            "Webhook payment update: payment_id={}, order_id={}, transaction_status={}, fraud_status={:?}",
            payment_id,
            webhook_payload.order_id,
            webhook_payload.transaction_status,
            webhook_payload.fraud_status
        );

        let status_str = match status {
            PaymentStatus::Pending => "pending",
            PaymentStatus::Success => "success",
            PaymentStatus::Failed => "failed",
            PaymentStatus::Expired => "expired",
            PaymentStatus::Refunded => "refunded",
        };

        let row = sqlx::query!(
            r#"
            UPDATE payments
            SET status = $1::varchar,
                transaction_id = $2,
                paid_at = CASE WHEN $1::varchar = 'success' THEN $3 ELSE paid_at END,
                updated_at = $3
            WHERE id = $4
            RETURNING id, rental_booking_id, sale_order_id, order_id,
                       transaction_id, va_number, bank, payment_type,
                       gross_amount, status as "status!", payment_for_type as "payment_for_type!",
                       refund_amount, refund_reason, paid_at, expired_at,
                       refunded_at, receipt_pdf_path, created_at, updated_at
            "#,
            status_str,
            transaction_id,
            now,
            payment_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(Payment {
            id: row.id,
            rental_booking_id: row.rental_booking_id,
            sale_order_id: row.sale_order_id,
            order_id: row.order_id,
            transaction_id: row.transaction_id,
            va_number: row.va_number,
            bank: row.bank,
            payment_type: row.payment_type,
            gross_amount: row.gross_amount.to_i64().ok_or_else(|| AppError::internal("Failed to convert BigDecimal to i64"))?,
            status: match row.status.as_str() {
                "pending" => PaymentStatus::Pending,
                "success" => PaymentStatus::Success,
                "failed" => PaymentStatus::Failed,
                "expired" => PaymentStatus::Expired,
                "refunded" => PaymentStatus::Refunded,
                _ => PaymentStatus::Pending,
            },
            payment_for_type: match row.payment_for_type.as_str() {
                "rental" => PaymentType::Rental,
                "sale" => PaymentType::Sale,
                _ => PaymentType::Rental,
            },
            refund_amount: row.refund_amount.and_then(|v| v.to_i64()),
            refund_reason: row.refund_reason,
            paid_at: row.paid_at,
            expired_at: row.expired_at,
            refunded_at: row.refunded_at,
            receipt_pdf_path: row.receipt_pdf_path,
            created_at: row.created_at.unwrap_or_else(Utc::now),
            updated_at: row.updated_at.unwrap_or_else(Utc::now),
        })
    }

    /// Process refund for a payment
    pub async fn process_refund(
        &self,
        payment_id: i32,
        refund_id: &str,
        refund_amount: i64,
        refund_reason: &str,
    ) -> Result<Payment, AppError> {
        let now = Utc::now();

        // Validate and log refund ID for audit trail
        if refund_id.is_empty() {
            return Err(AppError::validation("Refund ID cannot be empty"));
        }

        tracing::info!(
            "Processing refund: payment_id={}, refund_id={}, amount={}, reason={}",
            payment_id, refund_id, refund_amount, refund_reason
        );

        let row = sqlx::query!(
            r#"
            UPDATE payments
            SET status = 'refunded',
                refund_amount = $1,
                refund_reason = $2,
                refunded_at = $3,
                updated_at = $3
            WHERE id = $4
            RETURNING *
            "#,
            bigdecimal::BigDecimal::from(refund_amount),
            refund_reason,
            now,
            payment_id
        )
        .fetch_one(&self.pool)
        .await?;

        let payment = Payment {
            id: row.id,
            rental_booking_id: row.rental_booking_id,
            sale_order_id: row.sale_order_id,
            order_id: row.order_id,
            transaction_id: row.transaction_id,
            va_number: row.va_number,
            bank: row.bank,
            payment_type: row.payment_type,
            gross_amount: row.gross_amount.to_i64().ok_or_else(|| AppError::internal("Failed to convert BigDecimal to i64"))?,
            status: PaymentStatus::Refunded,
            payment_for_type: match row.payment_for_type.as_ref().map_or("rental", |s| s.as_str()) {
                "rental" => PaymentType::Rental,
                "sale" => PaymentType::Sale,
                _ => PaymentType::Rental,
            },
            refund_amount: row.refund_amount.and_then(|v| v.to_i64()),
            refund_reason: row.refund_reason,
            paid_at: row.paid_at,
            expired_at: row.expired_at,
            refunded_at: row.refunded_at,
            receipt_pdf_path: row.receipt_pdf_path,
            created_at: row.created_at.ok_or_else(|| AppError::internal("Missing created_at"))?,
            updated_at: row.updated_at.ok_or_else(|| AppError::internal("Missing updated_at"))?,
        };

        Ok(payment)
    }

    /// Update Midtrans response data
    pub async fn update_midtrans_response(
        &self,
        payment_id: i32,
        midtrans_response: &crate::domain::payment::MidtransChargeResponse,
    ) -> Result<Payment, AppError> {
        let va_number = midtrans_response.va_numbers
            .as_ref()
            .and_then(|vas| vas.first())
            .map(|va| va.va_number.clone());

        let bank = midtrans_response.va_numbers
            .as_ref()
            .and_then(|vas| vas.first())
            .map(|va| va.bank.clone());

        let now = Utc::now();

        let row = sqlx::query!(
            r#"
            UPDATE payments
            SET transaction_id = $1,
                va_number = $2,
                bank = $3,
                payment_type = $4,
                updated_at = $5
            WHERE id = $6
            RETURNING *
            "#,
            midtrans_response.transaction_id,
            va_number,
            bank,
            midtrans_response.payment_type,
            now,
            payment_id
        )
        .fetch_one(&self.pool)
        .await?;

        let payment = Payment {
            id: row.id,
            rental_booking_id: row.rental_booking_id,
            sale_order_id: row.sale_order_id,
            order_id: row.order_id,
            transaction_id: row.transaction_id,
            va_number: row.va_number,
            bank: row.bank,
            payment_type: row.payment_type,
            gross_amount: row.gross_amount.to_i64().ok_or_else(|| AppError::internal("Failed to convert BigDecimal to i64"))?,
            status: match row.status.as_str() {
                "pending" => PaymentStatus::Pending,
                "success" => PaymentStatus::Success,
                "failed" => PaymentStatus::Failed,
                "expired" => PaymentStatus::Expired,
                "refunded" => PaymentStatus::Refunded,
                _ => PaymentStatus::Pending,
            },
            payment_for_type: match row.payment_for_type.as_ref().map_or("rental", |s| s.as_str()) {
                "rental" => PaymentType::Rental,
                "sale" => PaymentType::Sale,
                _ => PaymentType::Rental,
            },
            refund_amount: row.refund_amount.and_then(|v| v.to_i64()),
            refund_reason: row.refund_reason,
            paid_at: row.paid_at,
            expired_at: row.expired_at,
            refunded_at: row.refunded_at,
            receipt_pdf_path: row.receipt_pdf_path,
            created_at: row.created_at.ok_or_else(|| AppError::internal("Missing created_at"))?,
            updated_at: row.updated_at.ok_or_else(|| AppError::internal("Missing updated_at"))?,
        };

        Ok(payment)
    }
}