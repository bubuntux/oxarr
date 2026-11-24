use anyhow::{anyhow, bail, Result};
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand_core::OsRng;

pub const MIN_PASSWORD_LEN: usize = 8;
pub const MAX_PASSWORD_LEN: usize = 128;
pub const MAX_USERNAME_LEN: usize = 64;

/// Normalizes and validates a username for account creation.
pub fn validate_username(raw: &str) -> Result<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        bail!("username cannot be empty");
    }
    if trimmed.chars().count() > MAX_USERNAME_LEN {
        bail!("username must be at most {MAX_USERNAME_LEN} characters");
    }
    if trimmed.chars().any(|c| c.is_control() || c.is_whitespace()) {
        bail!("username cannot contain spaces or control characters");
    }
    Ok(trimmed.to_owned())
}

/// Validates password length bounds without stripping characters.
pub fn validate_password(password: &str) -> Result<()> {
    let len = password.chars().count();
    if len < MIN_PASSWORD_LEN {
        bail!("password must be at least {MIN_PASSWORD_LEN} characters");
    }
    if len > MAX_PASSWORD_LEN {
        bail!("password must be at most {MAX_PASSWORD_LEN} characters");
    }
    Ok(())
}

/// Hashes a plaintext password using Argon2id with a fresh random salt.
pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|err| anyhow!("failed to hash password: {err}"))
}

/// Verifies a plaintext password against a stored Argon2id PHC string.
/// Unparseable or legacy hashes log a warning and return `false`.
pub fn verify_password(password: &str, password_hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(password_hash) else {
        tracing::warn!("stored password hash has invalid or legacy format");
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_hash_and_verify_round_trip() {
        let hash = hash_password("correct-horse-battery").expect("hash password");
        assert!(hash.starts_with("$argon2id$"));
        assert!(verify_password("correct-horse-battery", &hash));
        assert!(!verify_password("wrong-password", &hash));
    }

    #[test]
    fn hashing_uses_unique_random_salts() {
        let hash_a = hash_password("same-password").expect("hash A");
        let hash_b = hash_password("same-password").expect("hash B");
        assert_ne!(hash_a, hash_b);
    }

    #[test]
    fn invalid_or_legacy_hashes_fail_cleanly() {
        assert!(!verify_password("secret123", "sha256_secret123"));
        assert!(!verify_password("secret123", ""));
        assert!(!verify_password("secret123", "$invalid$format"));
    }

    #[test]
    fn validates_usernames_and_passwords() {
        assert_eq!(validate_username("  alice  ").unwrap(), "alice");
        assert!(validate_username("").is_err());
        assert!(validate_username("   ").is_err());
        assert!(validate_username("alice bob").is_err());
        assert!(validate_username(&"a".repeat(65)).is_err());

        assert!(validate_password("1234567").is_err());
        assert!(validate_password("12345678").is_ok());
        assert!(validate_password(&"a".repeat(129)).is_err());
    }
}
