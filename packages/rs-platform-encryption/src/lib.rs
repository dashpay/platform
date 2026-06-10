//! Cryptographic utilities for Dash Platform (DIP-15)
//!
//! This crate implements the Diffie-Hellman key exchange and encryption/decryption
//! operations as specified in DIP-15 for secure communication between Dash identities.

use aes::cipher::{block_padding::Pkcs7, KeyIvInit};
use aes::Aes256;
use dashcore::secp256k1::{PublicKey, SecretKey};

type Aes256CbcEnc = cbc::Encryptor<Aes256>;
type Aes256CbcDec = cbc::Decryptor<Aes256>;

/// Length of the DIP-15 compact extended-public-key plaintext, in bytes.
///
/// `parentFingerprint(4) ‖ chainCode(32) ‖ publicKey(33)` = 69 bytes. This is
/// the plaintext layout DIP-15 specifies for `encryptedPublicKey` and the form
/// both reference clients (iOS dash-shared-core, Android dashj) emit and
/// hard-check on receive. Encrypting exactly 69 bytes yields a 96-byte
/// ciphertext (16-byte IV + 80-byte AES-256-CBC/PKCS7 block), matching the
/// deployed contract's `minItems/maxItems: 96`.
pub const COMPACT_XPUB_LEN: usize = 69;

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
pub fn derive_shared_key_ecdh(private_key: &SecretKey, public_key: &PublicKey) -> [u8; 32] {
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
pub fn encrypt_aes_256_cbc(key: &[u8; 32], iv: &[u8; 16], data: &[u8]) -> Vec<u8> {
    use aes::cipher::BlockEncryptMut;

    let cipher = Aes256CbcEnc::new(key.into(), iv.into());
    let mut buffer = Vec::new();
    buffer.extend_from_slice(data);

    // Add padding
    let padding_needed = 16 - (data.len() % 16);
    buffer.resize(data.len() + padding_needed, padding_needed as u8);

    cipher
        .encrypt_padded_mut::<Pkcs7>(&mut buffer, data.len())
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
/// Encrypted extended public key with IV prepended (96 bytes: 16-byte IV + 80-byte encrypted data)
pub fn encrypt_extended_public_key(shared_key: &[u8; 32], iv: &[u8; 16], xpub: &[u8]) -> Vec<u8> {
    let encrypted_data = encrypt_aes_256_cbc(shared_key, iv, xpub);

    // Prepend IV to encrypted data as per DIP-15
    let mut result = Vec::with_capacity(16 + encrypted_data.len());
    result.extend_from_slice(iv);
    result.extend_from_slice(&encrypted_data);
    result
}

/// Decrypt an extended public key from DashPay contact requests (DIP-15)
///
/// # Arguments
/// * `shared_key` - 32-byte shared secret from ECDH
/// * `encrypted_data` - Encrypted extended public key with IV prepended (96 bytes total)
///
/// # Returns
/// Decrypted extended public key bytes
pub fn decrypt_extended_public_key(
    shared_key: &[u8; 32],
    encrypted_data: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    if encrypted_data.len() < 16 {
        return Err(CryptoError::InvalidCiphertextLength);
    }

    // Extract IV from first 16 bytes
    let iv: [u8; 16] = encrypted_data[..16].try_into().unwrap();
    let ciphertext = &encrypted_data[16..];

    decrypt_aes_256_cbc(shared_key, &iv, ciphertext)
}

/// Assemble the DIP-15 compact extended-public-key plaintext.
///
/// Concatenates `parent_fingerprint ‖ chain_code ‖ pubkey` into the 69-byte
/// compact form that DIP-15 defines for `encryptedPublicKey` (and that both
/// reference clients emit). This is the plaintext that should be fed to
/// [`encrypt_extended_public_key`] — *not* a BIP32/DIP-14 serialization, which
/// carries extra version/depth/child-number metadata the wire format omits.
///
/// # Arguments
/// * `parent_fingerprint` - 4-byte fingerprint of the parent key.
/// * `chain_code` - 32-byte chain code of the shared (account) key.
/// * `pubkey` - 33-byte compressed secp256k1 public key.
///
/// # Returns
/// The 69-byte compact plaintext.
pub fn compact_xpub_bytes(
    parent_fingerprint: [u8; 4],
    chain_code: [u8; 32],
    pubkey: [u8; 33],
) -> [u8; COMPACT_XPUB_LEN] {
    let mut out = [0u8; COMPACT_XPUB_LEN];
    out[0..4].copy_from_slice(&parent_fingerprint);
    out[4..36].copy_from_slice(&chain_code);
    out[36..69].copy_from_slice(&pubkey);
    out
}

/// Parse a DIP-15 compact extended-public-key plaintext back into its
/// three components.
///
/// Inverse of [`compact_xpub_bytes`]. Rejects any input whose length is not
/// exactly [`COMPACT_XPUB_LEN`] (69) bytes — the reference clients hard-check
/// this on receive, so a non-69-byte payload is not a valid DIP-15 compact
/// xpub and must be handled separately (e.g. a legacy 78/107-byte BIP32/DIP-14
/// serialization) by the caller.
///
/// # Arguments
/// * `bytes` - The decrypted plaintext (must be exactly 69 bytes).
///
/// # Returns
/// `(parent_fingerprint, chain_code, pubkey)` on success.
///
/// # Errors
/// [`CryptoError::InvalidCompactXpubLength`] if `bytes.len() != 69`.
#[allow(clippy::type_complexity)]
pub fn parse_compact_xpub(bytes: &[u8]) -> Result<([u8; 4], [u8; 32], [u8; 33]), CryptoError> {
    if bytes.len() != COMPACT_XPUB_LEN {
        return Err(CryptoError::InvalidCompactXpubLength(bytes.len()));
    }

    let mut parent_fingerprint = [0u8; 4];
    let mut chain_code = [0u8; 32];
    let mut pubkey = [0u8; 33];
    parent_fingerprint.copy_from_slice(&bytes[0..4]);
    chain_code.copy_from_slice(&bytes[4..36]);
    pubkey.copy_from_slice(&bytes[36..69]);

    Ok((parent_fingerprint, chain_code, pubkey))
}

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

/// Errors that can occur during cryptographic operations
#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("Decryption failed")]
    DecryptionFailed,

    #[error("Invalid UTF-8 in decrypted data")]
    InvalidUtf8,

    #[error("Invalid ciphertext length (must be at least 16 bytes for IV)")]
    InvalidCiphertextLength,

    #[error("Invalid compact xpub length (DIP-15 requires exactly 69 bytes, got {0})")]
    InvalidCompactXpubLength(usize),
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashcore::secp256k1::rand::{thread_rng, RngCore};
    use dashcore::secp256k1::Secp256k1;

    #[test]
    fn test_ecdh_key_derivation() {
        let secp = Secp256k1::new();

        // Generate two key pairs
        let (secret1, public1) = secp.generate_keypair(&mut thread_rng());
        let (secret2, public2) = secp.generate_keypair(&mut thread_rng());

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
        thread_rng().fill_bytes(&mut iv);

        let plaintext = b"Hello, DashPay!";

        let ciphertext = encrypt_aes_256_cbc(&key, &iv, plaintext);
        let decrypted = decrypt_aes_256_cbc(&key, &iv, &ciphertext).unwrap();

        assert_eq!(plaintext, decrypted.as_slice());
    }

    #[test]
    fn test_extended_public_key_encryption() {
        let secp = Secp256k1::new();
        let (secret1, _public1) = secp.generate_keypair(&mut thread_rng());
        let (_secret2, public2) = secp.generate_keypair(&mut thread_rng());

        // Derive shared key
        let shared_key = derive_shared_key_ecdh(&secret1, &public2);

        // Generate random IV
        let mut iv = [0u8; 16];
        thread_rng().fill_bytes(&mut iv);

        // DIP-15 compact xpub plaintext (69 bytes). 69 → PKCS7 → 80, + 16-byte
        // IV = exactly 96 bytes, matching the contract's minItems/maxItems: 96.
        let xpub_data = vec![0x04; COMPACT_XPUB_LEN];

        // Encrypt and decrypt
        let encrypted = encrypt_extended_public_key(&shared_key, &iv, &xpub_data);

        // Verify size: 16 bytes (IV) + 80 bytes (encrypted data) = 96 bytes
        assert_eq!(encrypted.len(), 96, "Encrypted xpub should be 96 bytes");

        let decrypted = decrypt_extended_public_key(&shared_key, &encrypted).unwrap();

        assert_eq!(xpub_data, decrypted);
    }

    #[test]
    fn test_compact_xpub_round_trip() {
        // Distinct byte patterns per field so a mis-sliced offset is caught.
        let parent_fingerprint = [0x11u8, 0x22, 0x33, 0x44];
        let chain_code = [0xAAu8; 32];
        let mut pubkey = [0xBBu8; 33];
        pubkey[0] = 0x02; // compressed-pubkey prefix

        let compact = compact_xpub_bytes(parent_fingerprint, chain_code, pubkey);
        assert_eq!(compact.len(), COMPACT_XPUB_LEN);
        assert_eq!(compact.len(), 69);

        // Byte-exact layout: fingerprint ‖ chaincode ‖ pubkey.
        assert_eq!(&compact[0..4], &parent_fingerprint);
        assert_eq!(&compact[4..36], &chain_code);
        assert_eq!(&compact[36..69], &pubkey);

        let (fp, cc, pk) = parse_compact_xpub(&compact).expect("parse 69-byte compact");
        assert_eq!(fp, parent_fingerprint);
        assert_eq!(cc, chain_code);
        assert_eq!(pk, pubkey);
    }

    #[test]
    fn test_encrypt_compact_xpub_is_exactly_96_bytes() {
        // The whole point of the 69-byte compact form: it encrypts to exactly
        // 96 bytes (16-byte IV + 80-byte AES-256-CBC/PKCS7), which is what the
        // deployed contract enforces. A 107-byte DIP-14 serialization would
        // yield 128 bytes and fail the contract's maxItems: 96.
        let shared_key = [0x07u8; 32];
        let iv = [0x09u8; 16];
        let plaintext = [0xCDu8; COMPACT_XPUB_LEN];

        let encrypted = encrypt_extended_public_key(&shared_key, &iv, &plaintext);
        assert_eq!(
            encrypted.len(),
            96,
            "69-byte compact plaintext must encrypt to exactly 96 bytes"
        );

        let decrypted = decrypt_extended_public_key(&shared_key, &encrypted).unwrap();
        assert_eq!(&decrypted[..], &plaintext[..]);
    }

    #[test]
    fn test_parse_compact_xpub_rejects_wrong_length() {
        // Lengths that are NOT 69 must be rejected — including the legacy 78/107
        // BIP32/DIP-14 serializations and the empty case.
        for bad_len in [0usize, 36, 68, 70, 78, 107, 128] {
            let bytes = vec![0u8; bad_len];
            let err = parse_compact_xpub(&bytes).expect_err("non-69-byte input must be rejected");
            assert!(
                matches!(err, CryptoError::InvalidCompactXpubLength(n) if n == bad_len),
                "expected InvalidCompactXpubLength({}), got {:?}",
                bad_len,
                err
            );
        }
    }

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
