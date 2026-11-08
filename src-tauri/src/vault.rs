use std::sync::atomic::{AtomicBool, Ordering};

use argon2::{
    password_hash::{phc::PasswordHash, PasswordHasher, PasswordVerifier},
    Argon2,
};

pub const MIN_PASSWORD_LEN: usize = 4;

/// Whether hidden folders are unlocked for this run of the app. Starts locked,
/// and is never persisted, so every launch asks for the password again.
#[derive(Default)]
pub struct Vault(AtomicBool);

impl Vault {
    pub fn is_unlocked(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }

    pub fn set_unlocked(&self, unlocked: bool) {
        self.0.store(unlocked, Ordering::SeqCst);
    }

    pub fn ensure_unlocked(&self) -> Result<(), String> {
        if self.is_unlocked() {
            Ok(())
        } else {
            Err("Desbloqueie as pastas ocultas para continuar.".to_string())
        }
    }
}

/// Hash a password into a salted Argon2id PHC string.
pub fn hash(password: &str) -> Result<String, String> {
    Argon2::default()
        .hash_password(password.as_bytes())
        .map(|h| h.to_string())
        .map_err(|e| e.to_string())
}

pub fn verify(password: &str, stored_hash: &str) -> bool {
    PasswordHash::new(stored_hash)
        .map(|parsed| Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifies_only_the_right_password() {
        let stored = hash("girassol").unwrap();
        assert!(verify("girassol", &stored));
        assert!(!verify("Girassol", &stored));
        assert!(!verify("girassol", "not a hash"));
    }
}
