//! Cryptographic utilities for Dash Platform (DIP-15)
//!
//! This crate implements the Diffie-Hellman key exchange and encryption/decryption
//! operations as specified in DIP-15 for secure communication between Dash identities.
//!
//! The DIP-15 surface is split by concern:
//! - [`ecdh`] — ECDH shared-secret derivation.
//! - [`aes`] — AES-256-CBC primitives shared by the encrypted fields.
//! - [`compact_xpub`] — the 69-byte compact xpub (`encryptedPublicKey`) + its encryption.
//! - [`account_label`] — `encryptedAccountLabel`.
//! - [`contact_info`] — `contactInfo` (`encToUserId` + `privateData`).
//! - [`account_reference`] — the masked `accountReference`.
//! - [`stealth`] — DIP-33 stealth one-time key derivation.
//! - [`error`] — the shared [`CryptoError`].
//!
//! Every public item is re-exported at the crate root, so the API is flat
//! (`platform_encryption::derive_shared_key_ecdh`, etc.) regardless of module.

mod account_label;
mod account_reference;
mod aes;
mod compact_xpub;
mod contact_info;
mod ecdh;
mod error;
mod stealth;

pub use account_label::{decrypt_account_label, encrypt_account_label};
pub use account_reference::{calculate_account_reference, unmask_account_reference};
pub use aes::{decrypt_aes_256_cbc, encrypt_aes_256_cbc};
pub use compact_xpub::{
    compact_xpub_bytes, decrypt_extended_public_key, encrypt_extended_public_key,
    parse_compact_xpub, CompactXpub, COMPACT_XPUB_LEN,
};
pub use contact_info::{
    decrypt_enc_to_user_id, decrypt_private_data, encrypt_enc_to_user_id, encrypt_private_data,
};
pub use ecdh::derive_shared_key_ecdh;
pub use error::CryptoError;
pub use stealth::{
    one_time_public_key, one_time_secret_key, one_time_tweak, stealth_shared_point, StealthRail,
};
