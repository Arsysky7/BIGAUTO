use regex::Regex;
use chrono::Datelike;

// Validate format email
pub fn is_valid_email(email: &str) -> bool {
    let email_regex = Regex::new(
        r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$"
    ).unwrap();

    email_regex.is_match(email)
}

// Validate nomor HP Indonesia (format: 08xxx atau +628xxx)
pub fn is_valid_phone(phone: &str) -> bool {
    let phone_regex = Regex::new(r"^(\+62|62|0)[8][0-9]{8,11}$").unwrap();
    phone_regex.is_match(phone)
}

// Validate nomor KTP Indonesia (16 digit)
pub fn is_valid_ktp(ktp: &str) -> bool {
    ktp.len() == 16 && ktp.chars().all(|c| c.is_numeric())
}

// Validate password strength (min 8 char, ada huruf & angka)
pub fn is_strong_password(password: &str) -> bool {
    if password.len() < 8 {
        return false;
    }

    let has_letter = password.chars().any(|c| c.is_alphabetic());
    let has_number = password.chars().any(|c| c.is_numeric());

    has_letter && has_number
}

// Validate price (harus positif, max 10 miliar)
pub fn is_valid_price(price: i64) -> bool {
    price > 0 && price <= 10_000_000_000
}

// Validate year (kendaraan, harus 1900-sekarang+1)
pub fn is_valid_year(year: i32) -> bool {
    let current_year = chrono::Utc::now().year();
    year >= 1900 && year <= current_year + 1
}

// Validate rating (1-5)
pub fn is_valid_rating(rating: i32) -> bool {
    (1..=5).contains(&rating)
}

// Sanitize string untuk prevent XSS
pub fn sanitize_html(input: &str) -> String {
    input
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_validation() {
        assert!(is_valid_email("test@example.com"));
        assert!(is_valid_email("user.name+tag@example.co.id"));
        assert!(!is_valid_email("invalid.email"));
        assert!(!is_valid_email("@example.com"));
    }

    #[test]
    fn test_phone_validation() {
        assert!(is_valid_phone("081234567890"));
        assert!(is_valid_phone("+628123456789"));
        assert!(is_valid_phone("628123456789"));
        assert!(!is_valid_phone("0712345678"));
        assert!(!is_valid_phone("12345"));
    }

    #[test]
    fn test_ktp_validation() {
        assert!(is_valid_ktp("1234567890123456"));
        assert!(!is_valid_ktp("123456789012345"));
        assert!(!is_valid_ktp("12345678901234567"));
        assert!(!is_valid_ktp("123456789012345a"));
    }

    #[test]
    fn test_password_strength() {
        assert!(is_strong_password("password123"));
        assert!(is_strong_password("Test1234"));
        assert!(!is_strong_password("short"));
        assert!(!is_strong_password("onlyletters"));
        assert!(!is_strong_password("12345678"));
    }

    #[test]
    fn test_price_validation() {
        assert!(is_valid_price(100_000));
        assert!(is_valid_price(5_000_000_000));
        assert!(!is_valid_price(0));
        assert!(!is_valid_price(-100));
        assert!(!is_valid_price(15_000_000_000));
    }

    #[test]
    fn test_rating_validation() {
        assert!(is_valid_rating(1));
        assert!(is_valid_rating(5));
        assert!(!is_valid_rating(0));
        assert!(!is_valid_rating(6));
    }
}
