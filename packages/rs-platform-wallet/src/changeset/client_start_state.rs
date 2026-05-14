//! Snapshot returned by [`PlatformWalletPersistence::load`] so the platform
//! wallet can boot without re-syncing from scratch.
//!
//! The struct is `#[non_exhaustive]` — adding a new sub-area in a future
//! release is source-incompatible only for code that destructures every
//! field, and downstream callers MUST initialise via `Default::default()`
//! and overwrite individual slots.
//!
//! [`PlatformWalletPersistence::load`]: crate::changeset::PlatformWalletPersistence::load

use std::collections::BTreeMap;

use dashcore::OutPoint;

use crate::changeset::client_wallet_start_state::ClientWalletStartState;
use crate::changeset::contacts_start_state::ContactsStartState;
use crate::changeset::identity_manager_start_state::IdentityManagerStartState;
use crate::changeset::platform_address_sync_start_state::PlatformAddressSyncStartState;
use crate::wallet::asset_lock::tracked::TrackedAssetLock;
use crate::wallet::platform_wallet::WalletId;

/// Snapshot of everything a persister hands back on
/// [`PlatformWalletPersistence::load`](crate::changeset::PlatformWalletPersistence::load)
/// so the platform wallet can boot without re-syncing from scratch.
///
/// Carries one slot per persisted sub-area. Empty maps mean "nothing was
/// persisted for that area"; the persister never substitutes defaults
/// for missing rows.
///
/// `wallets` stays empty until upstream provides a
/// `key_wallet::Wallet::from_persisted` constructor — the persister logs
/// a `wallets_pending_rehydration = N` event on every `load()` call so
/// operators can observe the gap.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct ClientStartState {
    /// Restored platform-address provider state per wallet — each
    /// value is passed directly to
    /// [`PlatformPaymentAddressProvider::from_persisted`](crate::wallet::platform_addresses::PlatformPaymentAddressProvider::from_persisted)
    /// so the provider can skip a full rescan.
    ///
    /// Persister-side reader:
    /// `platform_wallet_storage::sqlite::schema::platform_addrs::load_state`.
    pub platform_addresses: BTreeMap<WalletId, PlatformAddressSyncStartState>,
    /// Per-wallet startup slices (UTXOs and unused asset locks, each
    /// bucketed by account index).
    ///
    /// Empty pending upstream `Wallet::from_persisted`; see the type
    /// rustdoc.
    pub wallets: BTreeMap<WalletId, ClientWalletStartState>,
    /// Restored identity-manager state per wallet — passed to
    /// [`IdentityManager`](crate::wallet::identity::IdentityManager)
    /// reconstruction.
    ///
    /// Persister-side reader:
    /// `platform_wallet_storage::sqlite::schema::identities::load_state`.
    pub identities: BTreeMap<WalletId, IdentityManagerStartState>,
    /// Restored DashPay contact store per wallet.
    ///
    /// Persister-side reader:
    /// `platform_wallet_storage::sqlite::schema::contacts::load_state`.
    pub contacts: BTreeMap<WalletId, ContactsStartState>,
    /// Restored unused asset locks bucketed by `(wallet_id, account_index)`.
    /// Mirrors `ClientWalletStartState::unused_asset_locks` so the
    /// caller can fold each bucket into the live wallet at boot.
    ///
    /// Persister-side reader:
    /// `platform_wallet_storage::sqlite::schema::asset_locks::load_state`.
    pub asset_locks: BTreeMap<WalletId, BTreeMap<u32, BTreeMap<OutPoint, TrackedAssetLock>>>,
}

impl ClientStartState {
    /// `true` when no slot carries any rehydratable data.
    pub fn is_empty(&self) -> bool {
        self.platform_addresses.is_empty()
            && self.wallets.is_empty()
            && self.identities.is_empty()
            && self.contacts.is_empty()
            && self.asset_locks.is_empty()
    }
}
