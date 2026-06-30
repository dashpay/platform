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
use crate::manager::load_outcome::SkipReason;
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
    /// Wallets the persister itself rejected as structurally corrupt
    /// before they could be reconstructed (e.g. a malformed account xpub
    /// that aborts decode). They never appear in `wallets`; the manager
    /// folds them into the load outcome's `skipped` set and notifies
    /// handlers, so one bad persisted row never blocks the rest of the
    /// batch. Empty for persisters that decode every row up front.
    pub skipped: Vec<(WalletId, SkipReason)>,
    /// Restored shielded sub-wallet state — per-`SubwalletId`
    /// notes + sync watermarks. Consumed at `bind_shielded` time
    /// to rehydrate the in-memory `SubwalletState` so spending /
    /// balance reads work without re-decrypting the chain.
    #[cfg(feature = "shielded")]
    pub shielded: ShieldedSyncStartState,
}

impl ClientStartState {
    pub fn is_empty(&self) -> bool {
        // A skipped-only load (rows rejected, no wallets) is NOT empty:
        // the manager must still fire its skip notifications.
        let core_empty = self.platform_addresses.is_empty()
            && self.wallets.is_empty()
            && self.skipped.is_empty();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manager::load_outcome::CorruptKind;

    /// A skipped-only start state must report non-empty so the manager
    /// still surfaces `LoadOutcome::skipped` and fires skip handlers.
    #[test]
    fn skipped_only_state_is_not_empty() {
        let mut state = ClientStartState::default();
        assert!(state.is_empty(), "a freshly defaulted state is empty");

        state.skipped.push((
            [0u8; 32],
            SkipReason::CorruptPersistedRow {
                kind: CorruptKind::MalformedXpub,
            },
        ));
        assert!(
            !state.is_empty(),
            "a skipped-only state must be non-empty so skip notifications fire"
        );
    }
}
