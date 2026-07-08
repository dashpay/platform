//! Aggregate result of [`load_from_persistor`].
//!
//! [`load_from_persistor`]: super::PlatformWalletManager::load_from_persistor

use crate::wallet::platform_wallet::WalletId;

/// Why a persisted wallet row was passed over during a load pass.
///
/// Load is **watch-only** (no seed material involved): signing keys are
/// derived later, on demand, via the `MnemonicResolverHandle`
/// (`rs-sdk-ffi`) sign path. A skip therefore never means a wrong or
/// unavailable seed — the load path never touches one. Either the row
/// itself was unusable ([`CorruptPersistedRow`](Self::CorruptPersistedRow),
/// a per-row decode/structural failure) or the wallet was already
/// registered before this pass reached it
/// ([`AlreadyRegistered`](Self::AlreadyRegistered)). Variants carry no
/// key material (SECRETS.md SEC-REQ-2.0.1).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SkipReason {
    /// The persisted row could not be reconstructed: a structural decode
    /// failure on the keyless account manifest or carried snapshot.
    /// `kind` distinguishes the failure mode without leaking row bytes.
    #[error("persisted wallet row corrupt: {kind}")]
    CorruptPersistedRow {
        /// Structural family of the decode/projection failure.
        kind: CorruptKind,
    },
    /// The wallet was already registered before this load pass reached it
    /// (a prior load, or a wallet created at runtime), so its persisted
    /// row was not freshly loaded. Not corruption — it lets the caller
    /// tell an already-present wallet apart from one that genuinely loaded
    /// this pass.
    #[error("wallet already registered before this load pass")]
    AlreadyRegistered,
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
    /// The carried [`ManagedWalletInfo`] snapshot does not describe the
    /// persisted row it is attached to: its `wallet_id`/`network` differ
    /// from the row, or its account set diverges from the row's account
    /// manifest. This is a wrong-row/structurally-inconsistent snapshot —
    /// distinct from unreadable bytes ([`Self::DecodeError`]).
    ///
    /// [`ManagedWalletInfo`]: key_wallet::wallet::managed_wallet_info::ManagedWalletInfo
    SnapshotIdentityMismatch,
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
            Self::SnapshotIdentityMismatch => {
                f.write_str("snapshot does not match its persisted row")
            }
            Self::DecodeError(s) => write!(f, "decode error: {s}"),
            Self::ManifestIntegrityMismatch => f.write_str("manifest integrity mismatch"),
        }
    }
}

/// Aggregate, synchronous view of one
/// [`load_from_persistor`](super::PlatformWalletManager::load_from_persistor)
/// pass that did not hard-fail.
///
/// Three states, so the caller can tell a clean load from one that left
/// wallets behind without conflating either with a whole-load `Err`
/// (persister I/O, programmer error — that stays the `Err` arm of the
/// call). The load path is watch-only and never touches the seed, so no
/// wrong-seed outcome appears here.
///
/// Inspect the outcome via [`loaded`](Self::loaded) / [`skipped`](Self::skipped):
/// the skip reasons are what separate a harmless repeat
/// ([`AlreadyRegistered`](SkipReason::AlreadyRegistered)) from genuine
/// corruption ([`CorruptPersistedRow`](SkipReason::CorruptPersistedRow)) — a
/// distinction a flat `Result` shape would erase.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum LoadOutcome {
    /// Full success: every persisted wallet was reconstructed and
    /// registered. Also the empty-store first run — `loaded` is then
    /// empty and nothing was skipped.
    Loaded {
        /// Wallets reconstructed and registered, in load order.
        loaded: Vec<WalletId>,
    },
    /// Some wallets loaded, at least one was skipped (a corrupt row, or
    /// one already registered). The batch did not abort.
    Partial {
        /// Wallets reconstructed and registered, in load order.
        loaded: Vec<WalletId>,
        /// Wallets skipped, in load order.
        skipped: Vec<(WalletId, SkipReason)>,
    },
    /// Nothing usable: the persister returned rows but every one was
    /// skipped, so no wallet loaded even though the persister succeeded.
    NoneUsable {
        /// Wallets skipped, in load order.
        skipped: Vec<(WalletId, SkipReason)>,
    },
}

impl LoadOutcome {
    /// Pick the variant matching the `(loaded, skipped)` tallies of a pass.
    pub(crate) fn from_parts(loaded: Vec<WalletId>, skipped: Vec<(WalletId, SkipReason)>) -> Self {
        match (loaded.is_empty(), skipped.is_empty()) {
            (_, true) => Self::Loaded { loaded },
            (true, false) => Self::NoneUsable { skipped },
            (false, false) => Self::Partial { loaded, skipped },
        }
    }

    /// Wallets reconstructed and registered this pass, in load order.
    pub fn loaded(&self) -> &[WalletId] {
        match self {
            Self::Loaded { loaded } | Self::Partial { loaded, .. } => loaded,
            Self::NoneUsable { .. } => &[],
        }
    }

    /// Wallets skipped this pass (corrupt row, or already registered).
    pub fn skipped(&self) -> &[(WalletId, SkipReason)] {
        match self {
            Self::Partial { skipped, .. } | Self::NoneUsable { skipped } => skipped,
            Self::Loaded { .. } => &[],
        }
    }
}
