//! Bincode wire format for the Tier-2 envelope — `Envelope` /
//! `Payload` struct definitions.
//!
//! The encoder + decoder land in subsequent commits; T-1 stops at the
//! type definitions so the AAD encode-side tests compile against the
//! shared bincode config.

use crate::secrets::file::crypto::{NONCE_LEN, SALT_LEN};
use crate::secrets::wire::kdf::KdfParamsEncoded;

/// On-disk Tier-2 wire envelope. The whole struct is bincode-encoded
/// in one call; a wire-edited `version` is gated to
/// `SecretStoreError::UnsupportedEnvelopeVersion` before dispatch.
#[derive(bincode::Encode, bincode::Decode, Debug, PartialEq, Eq)]
pub(crate) struct Envelope {
    /// Envelope wire version (`ENVELOPE_VERSION`).
    pub version: u32,
    /// Tagged payload selecting unprotected vs password-protected.
    pub payload: Payload,
}

/// Tagged payload: scheme-0 ships the plaintext as-is (the backend's
/// own at-rest crypto is the only defence); scheme-1 ships the AEAD
/// triple under an object-password-derived key.
#[derive(bincode::Encode, bincode::Decode, Debug, PartialEq, Eq)]
pub(crate) enum Payload {
    /// Scheme 0 — unprotected passthrough; the bytes are the secret.
    Unprotected(Vec<u8>),
    /// Scheme 1 — sealed under an Argon2id-derived key with
    /// XChaCha20-Poly1305. The AAD bound at seal time is
    /// [`crate::secrets::wire::aad::Tier2Aad`].
    Password {
        /// Argon2 parameters used to derive the key.
        kdf: KdfParamsEncoded,
        /// Per-wrap CSPRNG salt fed into Argon2.
        salt: [u8; SALT_LEN],
        /// Per-wrap CSPRNG nonce fed into XChaCha20-Poly1305.
        nonce: [u8; NONCE_LEN],
        /// Ciphertext + 16-byte Poly1305 tag.
        ciphertext: Vec<u8>,
    },
}
