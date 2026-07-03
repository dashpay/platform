//! Aggregate result of [`load_from_persistor`].
//!
//! [`load_from_persistor`]: super::PlatformWalletManager::load_from_persistor

use crate::wallet::platform_wallet::WalletId;

/// Why a persisted wallet row was skipped during a load pass.
///
/// Load is **watch-only** (no seed material involved): signing keys are
/// derived later, on demand, via the `MnemonicResolverHandle`
/// (`rs-sdk-ffi`) sign path. A skip therefore means the persisted row
/// itself was unusable — a per-row decode/structural failure that fails
/// one wallet without aborting the batch. The only reason is
/// [`CorruptPersistedRow`](Self::CorruptPersistedRow): the load path
/// never touches the seed, so it cannot skip for a wrong or unavailable
/// seed. Variants carry no key material (SECRETS.md SEC-REQ-2.0.1).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SkipReason {
    /// The persisted row could not be reconstructed: a structural decode
    /// failure on the keyless account manifest or core-state projection.
    /// `kind` distinguishes the failure mode without leaking row bytes.
    #[error("persisted wallet row corrupt: {kind}")]
    CorruptPersistedRow {
        /// Structural family of the decode/projection failure.
        kind: CorruptKind,
    },
}

/// Structural family of [`SkipReason::CorruptPersistedRow`].
///
/// The variants are deliberately coarse — a finer split would require
/// the persister to round-trip backend error context that may carry
/// row-derived bytes. Apps drive their UI from the *family*, not from
/// the inner message.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CorruptKind {
    /// The wallet row exists but has no usable `AccountRegistrationEntry`
    /// manifest to rebuild the account collection from.
    MissingManifest,
    /// One or more manifest `account_xpub` bytes failed to parse as a
    /// well-formed extended public key.
    MalformedXpub,
    /// Any other structural decode / projection failure surfaced by the
    /// persister. The string is a structural projection — never a raw
    /// row byte slice or a hex-encoded key.
    DecodeError(String),
    /// A persisted account-manifest row failed its integrity checksum: the
    /// recomputed `SHA-256(wallet_id ‖ account_xpub_bytes)` did not match the
    /// stored value (a row bound to the wrong `wallet_id`, or a blob mutated in
    /// place). Tamper-evidence, not authentication — a local writer can forge
    /// it; the checksum only catches accidental corruption and migration bugs.
    ManifestIntegrityMismatch,
}

impl std::fmt::Display for CorruptKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingManifest => f.write_str("missing account manifest"),
            Self::MalformedXpub => f.write_str("malformed account xpub"),
            Self::DecodeError(s) => write!(f, "decode error: {s}"),
            Self::ManifestIntegrityMismatch => f.write_str("manifest integrity mismatch"),
        }
    }
}

/// Aggregate, synchronous view of one
/// [`load_from_persistor`](super::PlatformWalletManager::load_from_persistor)
/// pass.
///
/// `Ok(LoadOutcome)` with a non-empty `skipped` is **success** — a
/// per-row decode failure on one wallet is recorded and the batch
/// continues. The `Err` arm is reserved for whole-load failures
/// (persister I/O, programmer error). The load path is watch-only and
/// never touches the seed, so no wrong-seed outcome appears here.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct LoadOutcome {
    /// Wallets fully reconstructed and registered, in load order.
    pub loaded: Vec<WalletId>,
    /// Wallets skipped because their persisted row was corrupt, in load
    /// order.
    pub skipped: Vec<(WalletId, SkipReason)>,
}
