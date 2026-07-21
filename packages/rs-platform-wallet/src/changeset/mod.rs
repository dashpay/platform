//! Delta-based changesets for the platform wallet.
//!
//! This module provides:
//!
//! - [`Merge`] — a trait for composing changeset deltas.
//! - [`PlatformWalletChangeSet`] — the top-level delta type encompassing all wallet state.
//! - [`ClientStartState`] — the snapshot returned by
//!   [`PlatformWalletPersistence::load`] so the platform wallet can boot without re-syncing.
//! - [`PlatformWalletPersistence`] — storage backend trait.

#[allow(clippy::module_inception)]
pub mod changeset;
pub mod client_start_state;
pub mod client_wallet_start_state;
pub mod core_bridge;
pub mod identity_manager_start_state;
pub mod merge;
pub mod persistence_capabilities;
pub mod platform_address_sync_start_state;
#[cfg(feature = "serde")]
pub mod serde_adapters;
#[cfg(feature = "shielded")]
pub mod shielded_changeset;
#[cfg(feature = "shielded")]
pub mod shielded_sync_start_state;
pub mod traits;

pub(crate) use changeset::account_address_pool_entries;
#[cfg(any(feature = "bls", feature = "eddsa"))]
pub use changeset::{rebuild_provider_key_account, ProviderAccountRebuildError};
pub use changeset::{
    upsert_pending_contact_crypto, AccountAddressPoolEntry, AccountRegistrationEntry,
    AssetLockChangeSet, AssetLockEntry, ContactChangeSet, ContactRequestEntry, CoreChangeSet,
    HighestUsedIndexes, IdentityChangeSet, IdentityEntry, IdentityKeyDerivationIndices,
    IdentityKeyEntry, IdentityKeysChangeSet, InvitationChangeSet, InvitationEntry,
    InvitationStatus, KeyDerivationBreadcrumb, KeyWithBreadcrumb, PendingContactCrypto,
    PendingContactCryptoKey, PendingContactCryptoKind, PendingContactCryptoOp,
    PlatformAddressBalanceEntry, PlatformAddressChangeSet, PlatformWalletChangeSet,
    ProviderKeyAccountEntry, ProviderKeyExtendedPubKey, ProviderPlatformNodePubKey,
    ReceivedContactRequestKey, SentContactRequestKey, TokenBalanceChangeSet, WalletMetadataEntry,
};
pub use client_start_state::ClientStartState;
pub use client_wallet_start_state::ClientWalletStartState;
pub use core_bridge::spawn_wallet_event_adapter;
pub use identity_manager_start_state::IdentityManagerStartState;
pub use merge::Merge;
pub use persistence_capabilities::{PersistenceCapabilities, PERSISTENCE_CAPABILITIES_VERSION};
pub use platform_address_sync_start_state::PlatformAddressSyncStartState;
#[cfg(feature = "shielded")]
pub use shielded_changeset::ShieldedChangeSet;
#[cfg(feature = "shielded")]
pub use shielded_sync_start_state::{ShieldedSubwalletStartState, ShieldedSyncStartState};
pub use traits::{PersistenceError, PersistenceErrorKind, PlatformWalletPersistence};
