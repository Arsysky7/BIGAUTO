use rand::Rng;

// Generate a 6-digit OTP code (cryptographically secure)
pub fn generate_otp() -> String {
    let mut rng = rand::rng();
    let otp: u32 = rng.random_range(100_000..1_000_000);
    otp.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_otp_length() {
        let otp = generate_otp();
        assert_eq!(otp.len(), 6, "OTP harus tepat 6 digit");
    }

    #[test]
    fn test_otp_numeric() {
        let otp = generate_otp();
        assert!(
            otp.chars().all(|c| c.is_ascii_digit()),
            "OTP harus hanya mengandung digit numerik"
        );
    }

    #[test]
    fn test_otp_range() {
        let otp = generate_otp();
        let num: u32 = otp.parse().expect("OTP harus valid number");
        assert!(
            (100_000..=999_999).contains(&num),
            "OTP harus dalam rentang 100000 hingga 999999"
        );
    }

    #[test]
    fn test_otp_no_leading_zero() {
        let otp = generate_otp();
        assert!(
            !otp.starts_with('0'),
            "OTP tidak boleh diawali dengan angka nol untuk konsistensi 6 digit"
        );
    }

    #[test]
    fn test_otp_randomness() {
        let mut otps = HashSet::new();

        // Generate 1000 otp untuk test randomness
        for _ in 0..1000 {
            otps.insert(generate_otp());
        }

        // Minimal 98% dari OTP harus unik
        assert!(
            otps.len() >= 980,
            "OTP generation harus cukup random: got {} unique dari 1000", otps.len()
        );
    }
}