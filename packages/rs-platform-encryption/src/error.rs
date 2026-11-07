//! Error type shared across the DIP-15 crypto modules.

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
