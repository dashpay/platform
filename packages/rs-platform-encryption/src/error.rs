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

    #[error("a private document payload of {payload_len} bytes does not fit an entry that holds {capacity}")]
    PrivateDocumentPayloadTooLarge { payload_len: usize, capacity: usize },

    #[error("a private document entry of {0} bytes is shorter than the envelope's 299")]
    InvalidPrivateDocumentEntryLength(usize),

    #[error("the ephemeral seed derives no valid secret key; draw another")]
    InvalidPrivateDocumentEphemeralSeed,

    #[error("malformed private document entry: {0}")]
    MalformedPrivateDocumentEntry(&'static str),

    #[error("private document envelope version {0} is not supported")]
    UnsupportedPrivateDocumentEnvelopeVersion(u8),

    #[error("the private document entry names another recipient identity")]
    WrongPrivateDocumentRecipient,

    #[error("the private document entry is anonymous: it names no sender to check")]
    AnonymousPrivateDocumentSender,

    #[error("the private document entry's sender authentication does not verify")]
    PrivateDocumentSenderNotAuthenticated,
}
