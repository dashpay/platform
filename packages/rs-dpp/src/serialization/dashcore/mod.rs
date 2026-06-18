//! Serde `with` wrappers for types owned by external (dashcore) crates.
//!
//! These live here — rather than next to their dpp consumers — because the
//! type being (de)serialized is **not** a dpp type; it belongs to dashcore /
//! its transitive deps (e.g. `blstrs_plus`). Grouping them under
//! `serialization::dashcore` keeps "serde for out-of-crate types" in one place,
//! distinct from dpp-type-local serde (which lives next to its type).

/// Compressed-G1 BLS public key (`BlsPublicKey<Bls12381G2Impl>` from dashcore's
/// `blstrs_plus`). Used via `#[serde(with = "crate::serialization::dashcore::bls_pubkey")]`.
pub mod bls_pubkey;
