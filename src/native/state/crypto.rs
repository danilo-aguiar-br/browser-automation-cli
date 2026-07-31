// SPDX-License-Identifier: MIT OR Apache-2.0

use aes_gcm::{aead::Aead, aead::KeyInit, Aes256Gcm};
use sha2::{Digest, Sha256};

pub(crate) fn encrypt_data(data: &[u8], key_str: &str) -> Result<Vec<u8>, String> {
    use zeroize::Zeroize;

    let mut hasher = Sha256::new();
    hasher.update(key_str.as_bytes());
    // Materialize as owned array so we can zeroize after use (rules: secrets).
    let mut key_bytes: [u8; 32] = hasher.finalize().into();
    let result = (|| {
        let cipher =
            Aes256Gcm::new_from_slice(&key_bytes).map_err(|e| format!("Invalid key: {e}"))?;

        let mut nonce = [0u8; 12];
        getrandom::getrandom(&mut nonce).map_err(|e| format!("Failed to generate nonce: {e}"))?;
        let ciphertext = cipher
            .encrypt(aes_gcm::Nonce::from_slice(&nonce), data)
            .map_err(|e| format!("Encryption failed: {e}"))?;

        let mut result = Vec::with_capacity(12 + ciphertext.len());
        result.extend_from_slice(&nonce);
        result.extend_from_slice(&ciphertext);
        Ok(result)
    })();
    key_bytes.zeroize();
    result
}

pub(crate) fn decrypt_data(data: &[u8], key_str: &str) -> Result<Vec<u8>, String> {
    use zeroize::Zeroize;

    if data.len() < 13 {
        return Err("Ciphertext too short".to_string());
    }
    let (nonce_bytes, ciphertext) = data.split_at(12);

    let mut hasher = Sha256::new();
    hasher.update(key_str.as_bytes());
    let mut key_bytes: [u8; 32] = hasher.finalize().into();
    let result = (|| {
        let cipher =
            Aes256Gcm::new_from_slice(&key_bytes).map_err(|e| format!("Invalid key: {e}"))?;
        cipher
            .decrypt(aes_gcm::Nonce::from_slice(nonce_bytes), ciphertext)
            .map_err(|e| format!("Decryption failed: {e}"))
    })();
    key_bytes.zeroize();
    result
}
