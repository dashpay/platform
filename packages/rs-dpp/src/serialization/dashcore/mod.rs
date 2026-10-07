//! Serde adapters for the Core types embedded in Platform state.

/// Accept hex strings or byte sequences for Platform BLS public keys.
#[cfg(feature = "bls-signatures")]
pub mod bls_pubkey;
