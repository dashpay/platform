//! Compatibility re-exports for Serde field adapters.

/// Accept hex strings or byte sequences for Platform BLS public keys.
#[cfg(feature = "bls-signatures")]
#[deprecated(note = "use dpp::bls::serde instead")]
pub use crate::bls::serde as bls_pubkey;
