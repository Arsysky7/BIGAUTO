use crate::domain::payment::{
    CreatePaymentRequest, MidtransChargeRequest, TransactionDetails,
    BankTransfer, MidtransChargeResponse,
    MidtransWebhookPayload, PaymentStatus
};
use crate::error::AppError;
use reqwest::Client;
use hmac::{Hmac, Mac};
use sha2::Sha512;
use base64::{Engine as _, engine::general_purpose};


// Service untuk integrasi Midtrans
pub struct MidtransService {
    client: Client,
    server_key: String,
    is_production: bool,
    api_url: String,
}

type HmacSha512 = Hmac<Sha512>;

impl MidtransService {
    // Buat Midtrans Service baru
    pub fn new(
        server_key: String,
        _client_key: String,
        api_url: String,
    ) -> Self {
        let is_production = api_url.contains("api.midtrans.com");
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| AppError::internal(format!("Failed to create HTTP client: {}", e)))
            .unwrap();

        Self {
            client,
            server_key,
            is_production,
            api_url,
        }
    }

    /// Generate VA number unik
    fn generate_va_number(&self, order_id: &str, bank: &str) -> String {
        format!("{}-{}", bank.to_lowercase(), order_id)
    }

    // Convert request ke Midtrans format 
    fn convert_to_midtrans_request(
        &self,
        request: &CreatePaymentRequest,
        order_id: String,
    ) -> MidtransChargeRequest {
        let va_number = self.generate_va_number(&order_id, &request.payment_method);

        MidtransChargeRequest {
            payment_type: "bank_transfer".to_string(),
            transaction_details: TransactionDetails {
                order_id: order_id.clone(),
                gross_amount: request.gross_amount,
            },
            customer_details: request.customer_details.clone(),
            item_details: request.item_details.clone(),
            bank_transfer: Some(BankTransfer {
                bank: request.payment_method.clone(),
                va_number,
            }),
        }
    }

    // Charge payment ke Midtrans 
    pub async fn charge_payment(
        &self,
        request: &CreatePaymentRequest,
        order_id: String,
    ) -> Result<MidtransChargeResponse, AppError> {
        let midtrans_request = self.convert_to_midtrans_request(request, order_id);

        let auth_header = format!("Basic {}", self.encode_auth());

        let response = self.client
            .post(format!("{}/charge", self.api_url))
            .header("Authorization", auth_header)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&midtrans_request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AppError::midtrans(format!("Midtrans API error: {}", error_text)));
        }

        let midtrans_response: MidtransChargeResponse = response.json().await
            .map_err(|e| AppError::midtrans(format!("Failed to parse Midtrans response: {}", e)))?;

        Ok(midtrans_response)
    }

    /// Verify webhook signature dari Midtrans
    pub fn verify_webhook_signature(
        &self,
        payload: &str,
        signature: &str,
        order_id: &str,
    ) -> bool {
        let expected_signature = self.generate_signature(payload, order_id);
        expected_signature == signature
    }

    /// Generate signature untuk webhook
    fn generate_signature(&self, payload: &str, order_id: &str) -> String {
        let combined = format!("{}{}", order_id, payload);

        let mut mac = HmacSha512::new_from_slice(self.server_key.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(combined.as_bytes());

        let result = mac.finalize();
        let code_bytes = result.into_bytes();

        general_purpose::STANDARD.encode(code_bytes)
    }

    /// Encode auth credentials
    fn encode_auth(&self) -> String {
        let credentials = format!("{}:", self.server_key);
        general_purpose::STANDARD.encode(credentials.as_bytes())
    }

    /// Parse webhook payload
    pub fn parse_webhook_payload(&self, payload: &str) -> Result<MidtransWebhookPayload, AppError> {
        serde_json::from_str(payload)
            .map_err(|e| AppError::midtrans(format!("Failed to parse webhook payload: {}", e)))
    }

    /// Convert Midtrans status ke PaymentStatus
    pub fn convert_status(&self, transaction_status: &str) -> PaymentStatus {
        match transaction_status {
            "settlement" | "capture" => PaymentStatus::Success,
            "pending" => PaymentStatus::Pending,
            "deny" | "cancel" => PaymentStatus::Failed,
            "expire" => PaymentStatus::Expired,
            "refund" => PaymentStatus::Refunded,
            "partial_refund" => PaymentStatus::Refunded,
            _ => PaymentStatus::Failed,
        }
    }

    /// Get environment info for service endpoint
    pub fn get_environment_info(&self) -> String {
        if self.is_production {
            "production".to_string()
        } else {
            "sandbox".to_string()
        }
    }

    // Get payment instructions untuk customer
    pub fn get_payment_instructions(
        &self,
        bank: &str,
        va_number: &str
    ) -> String {
        match bank.to_lowercase().as_str() {
            "bca" => format!(
                "Silakan transfer ke BCA Virtual Account: {}\n\
                Melalui ATM BCA, KlikBCA, atau mobile banking.\n\
                Pembayaran akan diproses otomatis.",
                va_number
            ),
            "bni" => format!(
                "Silakan transfer ke BNI Virtual Account: {}\n\
                Melalui ATM BNI, BNI Mobile Banking, atau internet banking.\n\
                Pembayaran akan diproses otomatis.",
                va_number
            ),
            "mandiri" => format!(
                "Silakan transfer ke Mandiri Virtual Account: {}\n\
                Melalui ATM Mandiri, Mandiri Online, atau internet banking.\n\
                Pembayaran akan diproses otomatis.",
                va_number
            ),
            "bri" => format!(
                "Silakan transfer ke BRI Virtual Account: {}\n\
                Melalui ATM BRI, BRImo, atau internet banking.\n\
                Pembayaran akan diproses otomatis.",
                va_number
            ),
            "permata" => format!(
                "Silakan transfer ke Permata Virtual Account: {}\n\
                Melalui ATM Permata, PermataMobile X, atau internet banking.\n\
                Pembayaran akan diproses otomatis.",
                va_number
            ),
            _ => format!(
                "Silakan transfer ke Virtual Account {}: {}\n\
                Pembayaran akan diproses otomatis.",
                bank.to_uppercase(),
                va_number
            ),
        }
    }

    /// Check apakah environment production
    pub fn is_production(&self) -> bool {
        self.is_production
    }

  /// Check transaction status dari Midtrans API
  pub async fn check_transaction_status(&self, transaction_id: &str) -> Result<serde_json::Value, AppError> {
      let url = format!("{}/v2/{}/status", self.api_url, transaction_id);

      let response = self.client
          .get(&url)
          .header("Accept", "application/json")
          .header("Content-Type", "application/json")
          .basic_auth(&self.server_key, Some(""))
          .send()
          .await
          .map_err(|e| AppError::internal(format!("Failed to call Midtrans API: {}", e)))?;

      if response.status().is_success() {
          let status_response: serde_json::Value = response
              .json()
              .await
              .map_err(|e| AppError::internal(format!("Failed to parse Midtrans response: {}", e)))?;

          tracing::info!("✅ Midtrans status check successful for transaction: {}", transaction_id);
          Ok(status_response)
      } else {
          let error_text = response.text().await.unwrap_or_default();
          tracing::error!("❌ Midtrans status check failed: {}", error_text);
          Err(AppError::midtrans(format!("Midtrans API error: {}", error_text)))
      }
  }

  }