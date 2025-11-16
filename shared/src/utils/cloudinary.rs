use reqwest::multipart;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::env;

#[derive(Debug, Clone, Copy)]
pub enum ResourceType {
    Image,
    Raw,
    Video,
}

impl ResourceType {
    fn as_str(&self) -> &str {
        match self {
            ResourceType::Image => "image",
            ResourceType::Raw => "raw",
            ResourceType::Video => "video",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadResponse {
    pub secure_url: String,
    pub public_id: String,
    pub format: String,
    pub resource_type: String,
    pub bytes: i64,
    #[serde(default)]
    pub width: Option<i32>,
    #[serde(default)]
    pub height: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteResponse {
    pub result: String,
}

pub struct CloudinaryClient {
    cloud_name: String,
    api_key: String,
    api_secret: String,
}

impl CloudinaryClient {
    // Buat client dari environment variables
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let cloud_name = env::var("CLOUDINARY_CLOUD_NAME")?;
        let api_key = env::var("CLOUDINARY_API_KEY")?;
        let api_secret = env::var("CLOUDINARY_API_SECRET")?;

        Ok(Self {
            cloud_name,
            api_key,
            api_secret,
        })
    }

    // Upload file ke Cloudinary
    pub async fn upload(
        &self,
        bytes: Vec<u8>,
        resource_type: ResourceType,
        folder: &str,
        filename: Option<String>,
    ) -> Result<UploadResponse, Box<dyn std::error::Error>> {
        let url = self.build_upload_url(resource_type);
        let file_id = filename.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let form = multipart::Form::new()
            .text("api_key", self.api_key.clone())
            .text("folder", folder.to_string())
            .text("public_id", file_id)
            .part("file", multipart::Part::bytes(bytes).file_name("upload"));

        let response = reqwest::Client::new()
            .post(&url)
            .multipart(form)
            .send()
            .await?;

        if !response.status().is_success() {
            let err = response.text().await?;
            return Err(format!("Upload failed: {}", err).into());
        }

        Ok(response.json().await?)
    }

    // Upload image (shortcut)
    pub async fn upload_image(
        &self,
        bytes: Vec<u8>,
        folder: &str,
        filename: Option<String>,
    ) -> Result<UploadResponse, Box<dyn std::error::Error>> {
        self.upload(bytes, ResourceType::Image, folder, filename)
            .await
    }

    // Upload document (shortcut)
    pub async fn upload_document(
        &self,
        bytes: Vec<u8>,
        folder: &str,
        filename: Option<String>,
    ) -> Result<UploadResponse, Box<dyn std::error::Error>> {
        self.upload(bytes, ResourceType::Raw, folder, filename).await
    }

    // Generate URL dengan optimasi (resize, format, quality)
    pub fn optimized_url(
        &self,
        public_id: &str,
        width: Option<u32>,
        height: Option<u32>,
        quality: Option<&str>,
    ) -> String {
        let mut transforms = vec!["f_auto".to_string()];

        if let Some(q) = quality {
            transforms.push(format!("q_{}", q));
        }
        if let Some(w) = width {
            transforms.push(format!("w_{}", w));
        }
        if let Some(h) = height {
            transforms.push(format!("h_{}", h));
        }
        if width.is_some() && height.is_some() {
            transforms.push("c_fill".to_string());
        }

        format!(
            "https://res.cloudinary.com/{}/image/upload/{}/{}",
            self.cloud_name,
            transforms.join(","),
            public_id
        )
    }

    // Generate thumbnail URL
    pub fn thumbnail_url(&self, public_id: &str, size: Option<u32>) -> String {
        let dim = size.unwrap_or(150);
        self.optimized_url(public_id, Some(dim), Some(dim), Some("auto"))
    }

    // Hapus file dari Cloudinary
    pub async fn delete(
        &self,
        public_id: &str,
        resource_type: ResourceType,
    ) -> Result<DeleteResponse, Box<dyn std::error::Error>> {
        let url = self.build_delete_url(resource_type);
        let timestamp = chrono::Utc::now().timestamp();
        let signature = self.generate_signature(public_id, timestamp);

        let form = multipart::Form::new()
            .text("api_key", self.api_key.clone())
            .text("public_id", public_id.to_string())
            .text("timestamp", timestamp.to_string())
            .text("signature", signature);

        let response = reqwest::Client::new()
            .post(&url)
            .multipart(form)
            .send()
            .await?;

        if !response.status().is_success() {
            let err = response.text().await?;
            return Err(format!("Delete failed: {}", err).into());
        }

        Ok(response.json().await?)
    }

    // Hapus image (shortcut)
    pub async fn delete_image(
        &self,
        public_id: &str,
    ) -> Result<DeleteResponse, Box<dyn std::error::Error>> {
        self.delete(public_id, ResourceType::Image).await
    }

    // Hapus document (shortcut)
    pub async fn delete_document(
        &self,
        public_id: &str,
    ) -> Result<DeleteResponse, Box<dyn std::error::Error>> {
        self.delete(public_id, ResourceType::Raw).await
    }

    // Extract public_id dari Cloudinary URL
    pub fn extract_public_id(url: &str) -> Option<String> {
        if !url.contains("cloudinary.com") {
            return None;
        }

        let parts: Vec<&str> = url.split("/upload/").collect();
        if parts.len() != 2 {
            return None;
        }

        let after_upload = parts[1];
        let without_version = if after_upload.starts_with('v') {
            after_upload.split('/').skip(1).collect::<Vec<_>>().join("/")
        } else {
            after_upload.to_string()
        };

        without_version
            .rfind('.')
            .map(|idx| without_version[..idx].to_string())
    }

    // Build upload URL berdasarkan resource type
    fn build_upload_url(&self, resource_type: ResourceType) -> String {
        format!(
            "https://api.cloudinary.com/v1_1/{}/{}/upload",
            self.cloud_name,
            resource_type.as_str()
        )
    }

    // Build delete URL berdasarkan resource type
    fn build_delete_url(&self, resource_type: ResourceType) -> String {
        format!(
            "https://api.cloudinary.com/v1_1/{}/{}/destroy",
            self.cloud_name,
            resource_type.as_str()
        )
    }

    // Generate signature untuk authenticated requests
    fn generate_signature(&self, public_id: &str, timestamp: i64) -> String {
        let data = format!("public_id={}&timestamp={}{}", public_id, timestamp, self.api_secret);
        let mut hasher = Sha1::new();
        hasher.update(data.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_public_id() {
        let url = "https://res.cloudinary.com/test/image/upload/v123/vehicles/car-1.jpg";
        assert_eq!(CloudinaryClient::extract_public_id(url), Some("vehicles/car-1".to_string()));

        let url2 = "https://res.cloudinary.com/test/image/upload/profiles/user.png";
        assert_eq!(CloudinaryClient::extract_public_id(url2), Some("profiles/user".to_string()));
    }

    #[test]
    fn test_resource_type() {
        assert_eq!(ResourceType::Image.as_str(), "image");
        assert_eq!(ResourceType::Raw.as_str(), "raw");
        assert_eq!(ResourceType::Video.as_str(), "video");
    }
}
