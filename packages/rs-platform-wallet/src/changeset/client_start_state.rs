//! Snapshot returned by [`PlatformWalletPersistence::load`] so the platform
//! wallet can boot without re-syncing from scratch.
//!
//! Kept intentionally minimal — only the fields wired up today. The
//! rest of the sub-changesets (identities, contacts, token balances,
//! etc.) are deferred until their restore paths land.
//!
//! [`PlatformWalletPersistence::load`]: crate::changeset::PlatformWalletPersistence::load

use std::collections::BTreeMap;

use crate::changeset::client_wallet_start_state::ClientWalletStartState;
use crate::changeset::platform_address_sync_start_state::PlatformAddressSyncStartState;
#[cfg(feature = "shielded")]
use crate::changeset::shielded_sync_start_state::ShieldedSyncStartState;
use crate::wallet::platform_wallet::WalletId;

/// Snapshot of everything a persister hands back on
/// [`PlatformWalletPersistence::load`](crate::changeset::PlatformWalletPersistence::load)
/// so the platform wallet can boot without re-syncing from scratch.
///
/// Only carries the fields with an active restore path today. As new
/// areas gain persistence support (identities, contacts, token
/// balances, DashPay overlays, …), they will be added back here.
#[derive(Debug, Default)]
pub struct ClientStartState {
    /// Restored platform-address provider state per wallet — each
    /// value is passed directly to
    /// [`PlatformPaymentAddressProvider::from_persisted`](crate::wallet::platform_addresses::PlatformPaymentAddressProvider::from_persisted)
    /// so the provider can skip a full rescan.
    pub platform_addresses: BTreeMap<WalletId, PlatformAddressSyncStartState>,
    /// Per-wallet startup slices (UTXOs and unused asset locks, each
    /// bucketed by account index).
    pub wallets: BTreeMap<WalletId, ClientWalletStartState>,
    /// Restored shielded sub-wallet state — per-`SubwalletId`
    /// notes + sync watermarks. Consumed at `bind_shielded` time
    /// to rehydrate the in-memory `SubwalletState` so spending /
    /// balance reads work without re-decrypting the chain.
    #[cfg(feature = "shielded")]
    pub shielded: ShieldedSyncStartState,
}

impl ClientStartState {
    pub fn is_empty(&self) -> bool {
        let core_empty = self.platform_addresses.is_empty() && self.wallets.is_empty();
        #[cfg(feature = "shielded")]
        {
            core_empty && self.shielded.is_empty()
        }
        #[cfg(not(feature = "shielded"))]
        {
            core_empty
        }
    }
}
