use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2, Params, Version,
};

// Hash password menggunakan Argon2id dengan parameter OWASP 
pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);

    // OWASP recommended params: m=19456 (19MB), t=2, p=1
    let params = Params::new(19456, 2, 1, None)?;
    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, Version::V0x13, params);

    let hash = argon2.hash_password(password.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

// Verifikasi password dengan hash tersimpan menggunakan constant-time comparison
pub fn verify_password(password: &str, hash: &str) -> Result<bool, argon2::password_hash::Error> {
    let parsed_hash = PasswordHash::new(hash)?;
    let argon2 = Argon2::default();

    match argon2.verify_password(password.as_bytes(), &parsed_hash) {
        Ok(_) => Ok(true),
        Err(argon2::password_hash::Error::Password) => Ok(false),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify_success() {
        let password = "SecureP@ssw0rd123!";
        let hashed = hash_password(password).expect("Gagal hash password");

        assert_ne!(password, hashed);
        assert!(hashed.starts_with("$argon2id$"));
        assert!(verify_password(password, &hashed).expect("Gagal verify"));
    }

    #[test]
    fn test_verify_wrong_password() {
        let password = "CorrectPassword123";
        let wrong = "WrongPassword123";
        let hashed = hash_password(password).expect("Gagal hash");

        assert!(!verify_password(wrong, &hashed).expect("Gagal verify"));
    }

    #[test]
    fn test_different_hashes_same_password() {
        let password = "SamePassword@123";
        let hash1 = hash_password(password).expect("Gagal hash 1");
        let hash2 = hash_password(password).expect("Gagal hash 2");

        assert_ne!(hash1, hash2, "Hash harus berbeda karena random salt");
        assert!(verify_password(password, &hash1).unwrap());
        assert!(verify_password(password, &hash2).unwrap());
    }

    #[test]
    fn test_hash_format() {
        let password = "TestPassword123!";
        let hashed = hash_password(password).expect("Gagal hash");

        // Format: $argon2id$v=19$m=19456,t=2,p=1$salt$hash
        assert!(hashed.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"));
    }
}
