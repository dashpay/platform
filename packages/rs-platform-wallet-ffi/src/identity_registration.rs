//! FFI types shared by the identity-registration entry points.
//!
//! Historically this module also exposed
//! `platform_wallet_register_identity_from_addresses`, a mnemonic-driven
//! identity-registration FFI. That entry point has been deleted: the
//! `_with_signer` variant in
//! [`crate::identity_registration_with_signer`] supersedes it (the
//! mnemonic no longer crosses the FFI for identity registration; signing
//! is routed through external `SignerHandle`s instead — see
//! `swift-sdk/CLAUDE.md` for the architectural reasoning).
//!
//! The two flat input/output structs below survived the deletion
//! because the `_with_signer` FFI reuses them verbatim. Keeping them
//! here (rather than moving them next to the `_with_signer` fn) keeps
//! the public C ABI stable for any out-of-tree consumers that already
//! `#include` these struct shapes.

/// Flat input entry matching the SDK `put_with_address_funding`
/// shape. One row per contributing Platform Payment address.
///
/// Nonces are intentionally **not** part of this struct — the SDK's
/// auto-fetching variant resolves them from Platform right before
/// submit. Callers that need an explicit-nonce variant should use
/// the lower-level SDK API directly.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct IdentityFundingInputFFI {
    /// Address type discriminant (0 = P2PKH, 1 = P2SH). Matches the
    /// encoding used by `PlatformAddressFFI`.
    pub address_type: u8,
    /// 20-byte address hash.
    pub hash: [u8; 20],
    /// Credits to spend from this address for the new identity.
    pub credits: u64,
}

/// Optional refund / change output paired with the input list. When
/// `has_output` is false the remaining fields are ignored.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct IdentityFundingOutputFFI {
    pub has_output: bool,
    pub address_type: u8,
    pub hash: [u8; 20],
    pub credits: u64,
}
