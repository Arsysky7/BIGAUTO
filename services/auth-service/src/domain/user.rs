use crate::config::AppState;
use crate::error::AppError;
use crate::models::user::{UpdateUserProfile, User};
// Import validation utilities directly from submodule
use crate::utils::validation;

// Struktur data response profile user
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct ProfileResponse {
    #[schema(example = 1)]
    pub id: i32,
    #[schema(example = "john@example.com")]
    pub email: String,
    #[schema(example = "John Doe")]
    pub name: String,
    #[schema(example = "+6281234567890")]
    pub phone: String,
    #[schema(example = false)]
    pub is_seller: bool,
    #[schema(example = "Jl. Sudirman No. 123")]
    pub address: Option<String>,
    #[schema(example = "Jakarta")]
    pub city: Option<String>,
    #[schema(example = "https://example.com/photos/profile.jpg")]
    pub profile_photo: Option<String>,
    #[schema(example = "Toko Mobil Sejahtera")]
    pub business_name: Option<String>,
    #[schema(example = true)]
    pub email_verified: bool,
}

impl From<User> for ProfileResponse {
    fn from(user: User) -> Self {
        ProfileResponse {
            id: user.id,
            email: user.email,
            name: user.name,
            phone: user.phone,
            is_seller: user.is_seller.unwrap_or(false),
            address: user.address,
            city: user.city,
            profile_photo: user.profile_photo,
            business_name: user.business_name,
            email_verified: user.email_verified.unwrap_or(false),
        }
    }
}

// Struktur input untuk update profile
#[derive(Debug, serde::Deserialize)]
pub struct UpdateProfileInput {
    pub name: Option<String>,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub profile_photo: Option<String>,
}

// Struktur input untuk upgrade ke seller
#[derive(Debug, serde::Deserialize)]
pub struct UpgradeToSellerInput {
    pub business_name: String,
}

// Ambil profile user berdasarkan user_id 
pub async fn get_profile(
    state: &AppState,
    user_id: i32,
) -> Result<ProfileResponse, AppError> {
    let user = User::find_by_id(&state.db, user_id).await?
        .ok_or_else(|| AppError::NotFoundError("User tidak ditemukan".to_string()))?;
    
    Ok(user.into())
}

// Update profile user (name, phone, address, city, photo)
pub async fn update_profile(
    state: &AppState,
    user_id: i32,
    input: UpdateProfileInput,
) -> Result<ProfileResponse, AppError> {
    // Validasi phone jika diupdate
    if let Some(ref phone) = input.phone {
        validation::validate_phone(phone)
            .map_err(|e| AppError::ValidationError(e.to_string()))?;
    }

    // Validasi name tidak boleh kosong
    if let Some(ref name) = input.name {
        if name.trim().is_empty() {
            return Err(AppError::ValidationError(
                "Nama tidak boleh kosong".to_string(),
            ));
        }
    }

    // Prepare update data
    let update_data = UpdateUserProfile {
        name: input.name.map(|n| n.trim().to_string()),
        phone: input.phone.map(|p| validation::normalize_phone(&p)),
        address: input.address.map(|a| a.trim().to_string()),
        city: input.city.map(|c| c.trim().to_string()),
        profile_photo: input.profile_photo,
    };

    // Update di database
    User::update_profile(&state.db, user_id, update_data).await?;

    // Fetch user terbaru setelah update
    let updated_user = User::find_by_id(&state.db, user_id)
        .await?
        .ok_or_else(|| AppError::NotFoundError("User tidak ditemukan".to_string()))?;

    tracing::info!("Profile updated for user_id: {}", user_id);

    Ok(updated_user.into())
}

// Upgrade user menjadi seller dengan bussiness name 
pub async fn upgrade_to_seller(
    state: &AppState,
    user_id: i32,
    input: UpgradeToSellerInput,
) -> Result<ProfileResponse, AppError> {
    // Validasi business_name tidak boleh kosong
    if input.business_name.trim().is_empty() {
        return Err(AppError::ValidationError(
            "Nama bisnis tidak boleh kosong".to_string(),
        ));
    }

    // Load user untuk cek status
    let user = User::find_by_id(&state.db, user_id)
        .await?
        .ok_or_else(|| AppError::NotFoundError("User tidak ditemukan".to_string()))?;

    // Cek apakah sudah seller
    if user.is_seller.unwrap_or(false) {
        return Err(AppError::ConflictError(
            "Anda sudah terdaftar sebagai seller".to_string(),
        ));
    }

    // Upgrade ke seller
    let business_name = input.business_name.trim().to_string();
    User::upgrade_to_seller(&state.db, user_id, &business_name).await?;

    // Fetch user terbaru setelah upgrade
    let updated_user = User::find_by_id(&state.db, user_id)
        .await?
        .ok_or_else(|| AppError::NotFoundError("User tidak ditemukan".to_string()))?;

    tracing::info!(
        "User {} upgraded to seller with business: {}",
        user_id,
        business_name
    );

    Ok(updated_user.into())
}
