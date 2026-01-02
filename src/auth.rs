use tower_sessions::Session;

use crate::error::{MonitorError, Result};

/// Session key for storing authentication status
pub const SESSION_USER_KEY: &str = "authenticated";



/// Hash a plaintext password using bcrypt
pub fn hash_password(password: &str) -> Result<String> {
    bcrypt::hash(password, bcrypt::DEFAULT_COST)
        .map_err(|e| MonitorError::Authentication(format!("Failed to hash password: {}", e)))
}

/// Verify a plaintext password against a bcrypt hash
pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    bcrypt::verify(password, hash)
        .map_err(|e| MonitorError::Authentication(format!("Failed to verify password: {}", e)))
}


/// Check if the current session is authenticated
pub async fn is_authenticated(session: &Session) -> bool {
    session
        .get::<bool>(SESSION_USER_KEY)
        .await
        .ok()
        .flatten()
        .unwrap_or(false)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hashing() {
        let password = "test_password_123";
        let hash = hash_password(password).expect("Failed to hash password");

        // Hash should not be empty
        assert!(!hash.is_empty());

        // Hash should start with bcrypt prefix
        assert!(hash.starts_with("$2"));

        // Verify correct password
        assert!(verify_password(password, &hash).expect("Failed to verify password"));

        // Verify incorrect password
        assert!(!verify_password("wrong_password", &hash).expect("Failed to verify password"));
    }
}
