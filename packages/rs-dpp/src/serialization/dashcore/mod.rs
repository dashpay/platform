//! Serde `with` wrappers for types owned by external (dashcore) crates.
//!
//! These live here — rather than next to their dpp consumers — because the
//! type being (de)serialized is **not** a dpp type; it belongs to dashcore or
//! the BLS implementation (e.g. `blstrs_plus`). Grouping them under
//! `serialization::dashcore` keeps "serde for out-of-crate types" in one place,
//! distinct from dpp-type-local serde (which lives next to its type).

/// Compressed-G1 BLS public key (`BlsPublicKey<Bls12381G2Impl>` from `blsful`'s
/// `blstrs_plus`). Used via `#[serde(with = "crate::serialization::dashcore::bls_pubkey")]`.
///
/// The wrapped type comes from `crate::bls_signatures`, which is only linked when the
/// `bls-signatures` feature is on. Its only consumers —
/// the `core_types` validator/validator-set structs — are themselves gated behind
/// `core-types`, which requires `bls-signatures`, so gating here keeps the module
/// from breaking builds that omit BLS (e.g. `wasm-drive-verify`).
#[cfg(feature = "bls-signatures")]
pub mod bls_pubkey;
