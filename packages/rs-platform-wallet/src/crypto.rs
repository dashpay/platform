//! Cryptographic utilities for DashPay (DIP-15)
//!
//! This module implements the Diffie-Hellman key exchange and encryption/decryption
//! operations as specified in DIP-15 for secure communication between Dash identities.

use aes::Aes256;
use aes::cipher::{KeyIvInit, block_padding::Pkcs7};
use dashcore::secp256k1::{PublicKey, SecretKey};

type Aes256CbcEnc = cbc::Encryptor<Aes256>;
type Aes256CbcDec = cbc::Decryptor<Aes256>;

/// Derive a shared secret key using ECDH as specified in DIP-15
///
/// This uses libsecp256k1_ecdh which computes: SHA256((y[31]&0x1|0x2) || x)
/// where (x, y) is the EC point result of scalar multiplication
///
/// # Arguments
/// * `private_key` - The private key for this side of the exchange
/// * `public_key` - The public key from the other party
///
/// # Returns
/// A 32-byte shared secret key
pub fn derive_shared_key_ecdh(
    private_key: &SecretKey,
    public_key: &PublicKey,
) -> [u8; 32] {
    use dashcore::secp256k1::ecdh::SharedSecret;

    // Use secp256k1's built-in ECDH which matches libsecp256k1_ecdh
    // This computes SHA256((y[31]&0x1|0x2) || x) internally
    let shared_secret = SharedSecret::new(public_key, private_key);

    let mut key = [0u8; 32];
    key.copy_from_slice(shared_secret.as_ref());
    key
}

/// Encrypt data using CBC-AES-256
///
/// # Arguments
/// * `key` - 32-byte encryption key
/// * `iv` - 16-byte initialization vector (must be randomly generated and unique)
/// * `data` - Data to encrypt
///
/// # Returns
/// Encrypted data with PKCS7 padding
pub fn encrypt_aes_256_cbc(
    key: &[u8; 32],
    iv: &[u8; 16],
    data: &[u8],
) -> Vec<u8> {
    use aes::cipher::BlockEncryptMut;

    let cipher = Aes256CbcEnc::new(key.into(), iv.into());
    let mut buffer = Vec::new();
    buffer.extend_from_slice(data);

    // Add padding
    let padding_needed = 16 - (data.len() % 16);
    buffer.resize(data.len() + padding_needed, padding_needed as u8);

    cipher.encrypt_padded_mut::<Pkcs7>(&mut buffer, data.len())
        .expect("encryption failed")
        .to_vec()
}

/// Decrypt data using CBC-AES-256
///
/// # Arguments
/// * `key` - 32-byte encryption key
/// * `iv` - 16-byte initialization vector
/// * `ciphertext` - Encrypted data to decrypt
///
/// # Returns
/// Decrypted data with padding removed
pub fn decrypt_aes_256_cbc(
    key: &[u8; 32],
    iv: &[u8; 16],
    ciphertext: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    use aes::cipher::BlockDecryptMut;

    let cipher = Aes256CbcDec::new(key.into(), iv.into());
    let mut buffer = ciphertext.to_vec();

    let decrypted = cipher
        .decrypt_padded_mut::<Pkcs7>(&mut buffer)
        .map_err(|_| CryptoError::DecryptionFailed)?;

    Ok(decrypted.to_vec())
}

/// Encrypt an extended public key for DashPay contact requests (DIP-15)
///
/// # Arguments
/// * `shared_key` - 32-byte shared secret from ECDH
/// * `iv` - 16-byte initialization vector (must be randomly generated)
/// * `xpub` - Extended public key bytes to encrypt
///
/// # Returns
/// Encrypted extended public key (80 bytes with padding)
pub fn encrypt_extended_public_key(
    shared_key: &[u8; 32],
    iv: &[u8; 16],
    xpub: &[u8],
) -> Vec<u8> {
    encrypt_aes_256_cbc(shared_key, iv, xpub)
}

/// Decrypt an extended public key from DashPay contact requests (DIP-15)
///
/// # Arguments
/// * `shared_key` - 32-byte shared secret from ECDH
/// * `iv` - 16-byte initialization vector
/// * `ciphertext` - Encrypted extended public key
///
/// # Returns
/// Decrypted extended public key bytes
pub fn decrypt_extended_public_key(
    shared_key: &[u8; 32],
    iv: &[u8; 16],
    ciphertext: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    decrypt_aes_256_cbc(shared_key, iv, ciphertext)
}

/// Encrypt an account label for DashPay (DIP-15)
///
/// # Arguments
/// * `shared_key` - 32-byte shared secret from ECDH
/// * `iv` - 16-byte initialization vector (must be randomly generated, different from xpub IV)
/// * `label` - Account label string to encrypt
///
/// # Returns
/// Encrypted label (32-64 bytes with padding)
pub fn encrypt_account_label(
    shared_key: &[u8; 32],
    iv: &[u8; 16],
    label: &str,
) -> Vec<u8> {
    encrypt_aes_256_cbc(shared_key, iv, label.as_bytes())
}

/// Decrypt an account label from DashPay (DIP-15)
///
/// # Arguments
/// * `shared_key` - 32-byte shared secret from ECDH
/// * `iv` - 16-byte initialization vector
/// * `ciphertext` - Encrypted label
///
/// # Returns
/// Decrypted label string
pub fn decrypt_account_label(
    shared_key: &[u8; 32],
    iv: &[u8; 16],
    ciphertext: &[u8],
) -> Result<String, CryptoError> {
    let decrypted = decrypt_aes_256_cbc(shared_key, iv, ciphertext)?;
    String::from_utf8(decrypted).map_err(|_| CryptoError::InvalidUtf8)
}

/// Errors that can occur during cryptographic operations
#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("Decryption failed")]
    DecryptionFailed,

    #[error("Invalid UTF-8 in decrypted data")]
    InvalidUtf8,
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashcore::secp256k1::Secp256k1;
    use rand::RngCore;

    #[test]
    fn test_ecdh_key_derivation() {
        let secp = Secp256k1::new();

        // Generate two key pairs
        let (secret1, public1) = secp.generate_keypair(&mut rand::thread_rng());
        let (secret2, public2) = secp.generate_keypair(&mut rand::thread_rng());

        // Derive shared keys from both sides
        let shared1 = derive_shared_key_ecdh(&secret1, &public2);
        let shared2 = derive_shared_key_ecdh(&secret2, &public1);

        // Both sides should derive the same shared key
        assert_eq!(shared1, shared2);
    }

    #[test]
    fn test_aes_encryption_decryption() {
        let key = [0u8; 32];
        let mut iv = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut iv);

        let plaintext = b"Hello, DashPay!";

        let ciphertext = encrypt_aes_256_cbc(&key, &iv, plaintext);
        let decrypted = decrypt_aes_256_cbc(&key, &iv, &ciphertext).unwrap();

        assert_eq!(plaintext, decrypted.as_slice());
    }

    #[test]
    fn test_extended_public_key_encryption() {
        let secp = Secp256k1::new();
        let (secret1, _public1) = secp.generate_keypair(&mut rand::thread_rng());
        let (_secret2, public2) = secp.generate_keypair(&mut rand::thread_rng());

        // Derive shared key
        let shared_key = derive_shared_key_ecdh(&secret1, &public2);

        // Generate random IV
        let mut iv = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut iv);

        // Mock extended public key data
        let xpub_data = vec![0x04; 78]; // Example xpub size

        // Encrypt and decrypt
        let encrypted = encrypt_extended_public_key(&shared_key, &iv, &xpub_data);
        let decrypted = decrypt_extended_public_key(&shared_key, &iv, &encrypted).unwrap();

        assert_eq!(xpub_data, decrypted);
    }

    #[test]
    fn test_account_label_encryption() {
        let secp = Secp256k1::new();
        let (secret1, _public1) = secp.generate_keypair(&mut rand::thread_rng());
        let (_secret2, public2) = secp.generate_keypair(&mut rand::thread_rng());

        // Derive shared key
        let shared_key = derive_shared_key_ecdh(&secret1, &public2);

        // Generate random IV
        let mut iv = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut iv);

        let label = "My DashPay Account";

        // Encrypt and decrypt
        let encrypted = encrypt_account_label(&shared_key, &iv, label);
        let decrypted = decrypt_account_label(&shared_key, &iv, &encrypted).unwrap();

        assert_eq!(label, decrypted);
    }
}
