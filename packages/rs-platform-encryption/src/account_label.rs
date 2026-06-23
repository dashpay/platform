//! DIP-15 DashPay `encryptedAccountLabel` encryption.

use crate::aes::{decrypt_aes_256_cbc, encrypt_aes_256_cbc};
use crate::error::CryptoError;

/// Encrypt an account label for DashPay (DIP-15)
///
/// # Arguments
/// * `shared_key` - 32-byte shared secret from ECDH
/// * `iv` - 16-byte initialization vector (must be randomly generated, different from xpub IV)
/// * `label` - Account label string to encrypt
///
/// # Returns
/// Encrypted label with IV prepended (48-80 bytes: 16-byte IV + 32-64 byte encrypted data)
pub fn encrypt_account_label(shared_key: &[u8; 32], iv: &[u8; 16], label: &str) -> Vec<u8> {
    let encrypted_data = encrypt_aes_256_cbc(shared_key, iv, label.as_bytes());

    // Prepend IV to encrypted data as per DIP-15
    let mut result = Vec::with_capacity(16 + encrypted_data.len());
    result.extend_from_slice(iv);
    result.extend_from_slice(&encrypted_data);
    result
}

/// Decrypt an account label from DashPay (DIP-15)
///
/// # Arguments
/// * `shared_key` - 32-byte shared secret from ECDH
/// * `encrypted_data` - Encrypted label with IV prepended (48-80 bytes total)
///
/// # Returns
/// Decrypted label string
pub fn decrypt_account_label(
    shared_key: &[u8; 32],
    encrypted_data: &[u8],
) -> Result<String, CryptoError> {
    if encrypted_data.len() < 16 {
        return Err(CryptoError::InvalidCiphertextLength);
    }

    // Extract IV from first 16 bytes
    let iv: [u8; 16] = encrypted_data[..16].try_into().unwrap();
    let ciphertext = &encrypted_data[16..];

    let decrypted = decrypt_aes_256_cbc(shared_key, &iv, ciphertext)?;
    String::from_utf8(decrypted).map_err(|_| CryptoError::InvalidUtf8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecdh::derive_shared_key_ecdh;
    use dashcore::secp256k1::rand::{thread_rng, RngCore};
    use dashcore::secp256k1::Secp256k1;

    #[test]
    fn test_account_label_encryption() {
        let secp = Secp256k1::new();
        let (secret1, _public1) = secp.generate_keypair(&mut thread_rng());
        let (_secret2, public2) = secp.generate_keypair(&mut thread_rng());

        // Derive shared key
        let shared_key = derive_shared_key_ecdh(&secret1, &public2);

        // Generate random IV
        let mut iv = [0u8; 16];
        thread_rng().fill_bytes(&mut iv);

        let label = "My DashPay Account";

        // Encrypt and decrypt
        let encrypted = encrypt_account_label(&shared_key, &iv, label);

        // Verify size is in valid range: 48-80 bytes (16-byte IV + 32-64 bytes encrypted)
        assert!(
            encrypted.len() >= 48 && encrypted.len() <= 80,
            "Encrypted label should be 48-80 bytes, got {}",
            encrypted.len()
        );

        let decrypted = decrypt_account_label(&shared_key, &encrypted).unwrap();

        assert_eq!(label, decrypted);
    }
}
