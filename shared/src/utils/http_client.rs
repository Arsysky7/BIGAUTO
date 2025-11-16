use reqwest::{Client, Response, StatusCode};
use serde::de::DeserializeOwned;
use std::env;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HttpClientError {
    #[error("Request failed: {0}")]
    RequestFailed(String),

    #[error("Service tidak tersedia: {0}")]
    ServiceUnavailable(String),

    #[error("Response parsing error: {0}")]
    ParseError(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),
}

pub struct ServiceClient {
    client: Client,
    base_url: String,
}

impl ServiceClient {
    // Buat client untuk service tertentu
    pub fn new(service_name: &str) -> Result<Self, HttpClientError> {
        let base_url = match service_name {
            "auth" => env::var("AUTH_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:3001".to_string()),
            "user" => env::var("USER_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:3002".to_string()),
            "vehicle" => env::var("VEHICLE_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:3003".to_string()),
            "booking" => env::var("BOOKING_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:3004".to_string()),
            "payment" => env::var("PAYMENT_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:3005".to_string()),
            "chat" => env::var("CHAT_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:3006".to_string()),
            "notification" => env::var("NOTIFICATION_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:3007".to_string()),
            "financial" => env::var("FINANCIAL_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:3008".to_string()),
            _ => return Err(HttpClientError::ServiceUnavailable(
                format!("Unknown service: {}", service_name)
            )),
        };

        Ok(Self {
            client: Client::new(),
            base_url,
        })
    }

    // GET request dengan authentication
    pub async fn get<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        token: Option<&str>,
    ) -> Result<T, HttpClientError> {
        let url = format!("{}{}", self.base_url, endpoint);
        let mut request = self.client.get(&url);

        if let Some(t) = token {
            request = request.header("Authorization", format!("Bearer {}", t));
        }

        let response = request
            .send()
            .await
            .map_err(|e| HttpClientError::RequestFailed(e.to_string()))?;

        self.handle_response(response).await
    }

    // POST request dengan authentication
    pub async fn post<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        endpoint: &str,
        body: &B,
        token: Option<&str>,
    ) -> Result<T, HttpClientError> {
        let url = format!("{}{}", self.base_url, endpoint);
        let mut request = self.client.post(&url).json(body);

        if let Some(t) = token {
            request = request.header("Authorization", format!("Bearer {}", t));
        }

        let response = request
            .send()
            .await
            .map_err(|e| HttpClientError::RequestFailed(e.to_string()))?;

        self.handle_response(response).await
    }

    // PUT request dengan authentication
    pub async fn put<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        endpoint: &str,
        body: &B,
        token: Option<&str>,
    ) -> Result<T, HttpClientError> {
        let url = format!("{}{}", self.base_url, endpoint);
        let mut request = self.client.put(&url).json(body);

        if let Some(t) = token {
            request = request.header("Authorization", format!("Bearer {}", t));
        }

        let response = request
            .send()
            .await
            .map_err(|e| HttpClientError::RequestFailed(e.to_string()))?;

        self.handle_response(response).await
    }

    // DELETE request dengan authentication
    pub async fn delete(
        &self,
        endpoint: &str,
        token: Option<&str>,
    ) -> Result<(), HttpClientError> {
        let url = format!("{}{}", self.base_url, endpoint);
        let mut request = self.client.delete(&url);

        if let Some(t) = token {
            request = request.header("Authorization", format!("Bearer {}", t));
        }

        let response = request
            .send()
            .await
            .map_err(|e| HttpClientError::RequestFailed(e.to_string()))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(HttpClientError::RequestFailed(
                format!("Status: {}", response.status())
            ))
        }
    }

    // Handle response dan parse JSON
    async fn handle_response<T: DeserializeOwned>(
        &self,
        response: Response,
    ) -> Result<T, HttpClientError> {
        let status = response.status();

        match status {
            StatusCode::OK | StatusCode::CREATED => {
                response
                    .json::<T>()
                    .await
                    .map_err(|e| HttpClientError::ParseError(e.to_string()))
            }
            StatusCode::UNAUTHORIZED => {
                Err(HttpClientError::Unauthorized("Token invalid".to_string()))
            }
            _ => {
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                Err(HttpClientError::RequestFailed(error_text))
            }
        }
    }
}

// Helper functions untuk quick access
pub async fn get_user_profile(user_id: i32, token: &str) -> Result<serde_json::Value, HttpClientError> {
    let client = ServiceClient::new("user")?;
    client.get(&format!("/api/users/{}", user_id), Some(token)).await
}

pub async fn get_vehicle(vehicle_id: i32) -> Result<serde_json::Value, HttpClientError> {
    let client = ServiceClient::new("vehicle")?;
    client.get(&format!("/api/vehicles/{}", vehicle_id), None).await
}

pub async fn check_vehicle_availability(
    vehicle_id: i32,
    start_date: &str,
    end_date: &str,
) -> Result<bool, HttpClientError> {
    let client = ServiceClient::new("vehicle")?;
    let response: serde_json::Value = client
        .get(
            &format!("/api/vehicles/{}/availability?start={}&end={}", vehicle_id, start_date, end_date),
            None,
        )
        .await?;

    Ok(response["available"].as_bool().unwrap_or(false))
}
