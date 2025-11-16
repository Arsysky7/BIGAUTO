use regex::Regex;
use once_cell::sync::Lazy;

// Regex untuk email validation (RFC 5322 compliant)
static EMAIL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$")
        .unwrap()
});

// Regex untuk nomor telepon Indonesia (08xx, 628xx, atau +628xx format)
static PHONE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(\+62|62|0)8[1-9][0-9]{7,11}$")
        .unwrap()
});

// Validasi format email sesuai RFC 5322 standard
pub fn validate_email(email: &str) -> Result<(), String> {
    let trimmed = email.trim();

    if trimmed.is_empty() {
        return Err("Email tidak boleh kosong".to_string());
    }

    if trimmed.len() > 254 {
        return Err("Email terlalu panjang (maksimal 254 karakter)".to_string());
    }

    if !EMAIL_REGEX.is_match(trimmed) {
        return Err("Format email tidak valid".to_string());
    }

    Ok(())
}

// Validasi password dengan aturan keamanan enterprise-grade
pub fn validate_password(password: &str) -> Result<(), String> {
    if password.len() < 8 {
        return Err("Password minimal 8 karakter".to_string());
    }

    if password.len() > 128 {
        return Err("Password maksimal 128 karakter".to_string());
    }

    let has_uppercase = password.chars().any(|c| c.is_uppercase());
    let has_lowercase = password.chars().any(|c| c.is_lowercase());
    let has_digit = password.chars().any(|c| c.is_numeric());

    if !has_uppercase {
        return Err("Password harus mengandung minimal 1 huruf besar".to_string());
    }

    if !has_lowercase {
        return Err("Password harus mengandung minimal 1 huruf kecil".to_string());
    }

    if !has_digit {
        return Err("Password harus mengandung minimal 1 angka".to_string());
    }

    Ok(())
}

// Validasi nomor telepon Indonesia dengan berbagai format
pub fn validate_phone(phone: &str) -> Result<(), String> {
    let trimmed = phone.trim().replace(&[' ', '-'][..], "");

    if trimmed.is_empty() {
        return Err("Nomor telepon tidak boleh kosong".to_string());
    }

    if !PHONE_REGEX.is_match(&trimmed) {
        return Err("Format nomor telepon tidak valid (gunakan format 08xx, 628xx, atau +628xx)".to_string());
    }

    Ok(())
}

// Normalisasi nomor telepon ke format +628xx untuk konsistensi database
pub fn normalize_phone(phone: &str) -> String {
    let cleaned = phone.trim().replace(&[' ', '-'][..], "");

    if cleaned.starts_with("+62") {
        cleaned
    } else if cleaned.starts_with("62") {
        format!("+{}", cleaned)
    } else if cleaned.starts_with('0') {
        format!("+62{}", &cleaned[1..])
    } else {
        cleaned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_emails() {
        assert!(validate_email("user@example.com").is_ok());
        assert!(validate_email("test.email@domain.co.id").is_ok());
        assert!(validate_email("user+tag@example.com").is_ok());
        assert!(validate_email("123@test.com").is_ok());
    }

    #[test]
    fn test_invalid_emails() {
        assert!(validate_email("").is_err());
        assert!(validate_email("invalid").is_err());
        assert!(validate_email("@example.com").is_err());
        assert!(validate_email("user@").is_err());
        assert!(validate_email("user @example.com").is_err());
    }

    #[test]
    fn test_valid_passwords() {
        assert!(validate_password("Password123").is_ok());
        assert!(validate_password("SecureP@ss1").is_ok());
        assert!(validate_password("MyPass123").is_ok());
    }

    #[test]
    fn test_invalid_passwords() {
        assert!(validate_password("short1A").is_err()); 
        assert!(validate_password("alllowercase123").is_err()); 
        assert!(validate_password("ALLUPPERCASE123").is_err()); 
        assert!(validate_password("NoDigitsHere").is_err()); 
    }

    #[test]
    fn test_valid_phones() {
        assert!(validate_phone("08123456789").is_ok());
        assert!(validate_phone("628123456789").is_ok());
        assert!(validate_phone("+628123456789").is_ok());
        assert!(validate_phone("0812-3456-7890").is_ok()); 
    }

    #[test]
    fn test_invalid_phones() {
        assert!(validate_phone("").is_err());
        assert!(validate_phone("123456").is_err()); 
        assert!(validate_phone("07123456789").is_err()); 
        assert!(validate_phone("8123456789").is_err()); 
    }

    #[test]
    fn test_normalize_phone() {
        assert_eq!(normalize_phone("08123456789"), "+628123456789");
        assert_eq!(normalize_phone("628123456789"), "+628123456789");
        assert_eq!(normalize_phone("+628123456789"), "+628123456789");
        assert_eq!(normalize_phone("0812-3456-7890"), "+6281234567890");
    }
}
