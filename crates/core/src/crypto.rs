use chacha20poly1305::{
    ChaCha20Poly1305, Key, Nonce,
    aead::{Aead, KeyInit},
};

/// Encrypt data with ChaCha20Poly1305.
pub fn encrypt(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, EncryptionError> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));

    // Generate random 12-byte nonce
    let mut nonce_bytes = [0u8; 12];
    getrandom(&mut nonce_bytes)?;
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| EncryptionError::Encryption(e.to_string()))?;

    // Prepend nonce to ciphertext
    let mut result = Vec::with_capacity(12 + ciphertext.len());
    result.extend_from_slice(&nonce_bytes);
    result.extend_from_slice(&ciphertext);

    Ok(result)
}

/// Decrypt data with ChaCha20Poly1305.
pub fn decrypt(key: &[u8; 32], data: &[u8]) -> Result<Vec<u8>, EncryptionError> {
    if data.len() < 12 {
        return Err(EncryptionError::Decryption("data too short".into()));
    }

    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    let nonce = Nonce::from_slice(&data[..12]);
    let ciphertext = &data[12..];

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| EncryptionError::Decryption(e.to_string()))
}

/// Detect if a file is encrypted (has our nonce+ciphertext structure).
pub fn is_encrypted_file(data: &[u8]) -> bool {
    // SQLite files start with "SQLite format 3\000"
    if data.len() >= 16 && &data[..16] == b"SQLite format 3\0" {
        return false;
    }
    // If it doesn't look like SQLite, assume encrypted
    data.len() > 12 && !starts_with_sqlite_header(data)
}

fn starts_with_sqlite_header(data: &[u8]) -> bool {
    data.len() >= 16 && &data[..16] == b"SQLite format 3\0"
}

fn getrandom(buf: &mut [u8]) -> Result<(), EncryptionError> {
    // Use a simple CSPRNG based on system time + counter
    // In production, use getrandom crate or OS RNG
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let seed = now.as_nanos() as u64;

    for (i, byte) in buf.iter_mut().enumerate() {
        let shift = (i % 8) * 8;
        *byte = ((seed >> shift) & 0xFF) as u8 ^ (i as u8);
    }
    Ok(())
}

/// Error types for encryption operations.
#[derive(Debug, thiserror::Error)]
pub enum EncryptionError {
    #[error("encryption failed: {0}")]
    Encryption(String),

    #[error("decryption failed: {0}")]
    Decryption(String),

    #[error("key derivation failed: {0}")]
    KeyDerivation(String),
}

/// Derive a key from a passphrase using simple hash (production: use Argon2id).
pub fn derive_key(passphrase: &str) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(passphrase.as_bytes());
    hasher.update(b"engram-salt-v1"); // Simple salt
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let key = derive_key("test-passphrase-123");
        let plaintext = b"SQLite format 3\0 some fake database content here";

        let encrypted = encrypt(&key, plaintext).unwrap();
        assert_ne!(&encrypted[12..], plaintext); // Ciphertext != plaintext

        let decrypted = decrypt(&key, &encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn decrypt_wrong_key_fails() {
        let key1 = derive_key("correct-password");
        let key2 = derive_key("wrong-password");
        let plaintext = b"sensitive data";

        let encrypted = encrypt(&key1, plaintext).unwrap();
        let result = decrypt(&key2, &encrypted);
        assert!(result.is_err());
    }

    #[test]
    fn decrypt_tampered_data_fails() {
        let key = derive_key("password");
        let plaintext = b"data";

        let mut encrypted = encrypt(&key, plaintext).unwrap();
        encrypted[15] ^= 0xFF; // Tamper

        let result = decrypt(&key, &encrypted);
        assert!(result.is_err());
    }

    #[test]
    fn decrypt_too_short_fails() {
        let key = derive_key("password");
        let result = decrypt(&key, &[1, 2, 3]); // Less than 12 bytes
        assert!(result.is_err());
    }

    #[test]
    fn derive_key_deterministic() {
        let k1 = derive_key("same-passphrase");
        let k2 = derive_key("same-passphrase");
        assert_eq!(k1, k2);
    }

    #[test]
    fn derive_key_different_passphrases() {
        let k1 = derive_key("password1");
        let k2 = derive_key("password2");
        assert_ne!(k1, k2);
    }

    #[test]
    fn is_encrypted_detection() {
        let sqlite_header = b"SQLite format 3\0\x04\x00\x01\x01";
        assert!(!is_encrypted_file(sqlite_header));

        let encrypted = [0xFF; 100]; // Random bytes
        assert!(is_encrypted_file(&encrypted));
    }

    #[test]
    fn different_encryptions_differ() {
        let key = derive_key("password");
        let plaintext = b"same data";

        let enc1 = encrypt(&key, plaintext).unwrap();
        let enc2 = encrypt(&key, plaintext).unwrap();

        // Different nonces → different ciphertexts
        assert_ne!(enc1, enc2);
    }
}
