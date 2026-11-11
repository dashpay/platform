//! Snapshot returned by [`PlatformWalletPersistence::load`] so the platform
//! wallet can boot without re-syncing from scratch.
//!
//! Kept intentionally minimal — only the fields wired up today. The
//! rest of the sub-changesets (identities, contacts, token balances,
//! etc.) are deferred until their restore paths land.
//!
//! [`PlatformWalletPersistence::load`]: crate::changeset::PlatformWalletPersistence::load

use std::collections::BTreeMap;

use dpp::prelude::Identifier;

use crate::changeset::changeset::PrivateContactRequestsPosition;
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
    /// The private contact request scan state records
    /// (`ContactChangeSet::private_contact_requests_positions`) the backend
    /// kept for wallets it does not return in [`Self::wallets`], keyed by
    /// wallet id, then owner identity: those of a removed wallet whose
    /// marker (`PrivateContactRequestsPosition::private_payments_unresolved`)
    /// is set. A backend that removes a wallet keeps each such record until a
    /// record stored cleared for that wallet and owner replaces it, as a
    /// private payment it stands for may still execute.
    ///
    /// Only the marker is used: an identity added to a wallet registered or
    /// loaded under that id is held, as one loaded with its record marked is,
    /// in every build (`PlatformWalletError::PrivateContactPaymentsNotRestored`).
    /// The read position is not, as the requests read before it went with the
    /// wallet's other rows.
    pub retained_private_contact_scan_states:
        BTreeMap<WalletId, BTreeMap<Identifier, PrivateContactRequestsPosition>>,
}

impl ClientStartState {
    pub fn is_empty(&self) -> bool {
        let core_empty = self.platform_addresses.is_empty()
            && self.wallets.is_empty()
            && self.retained_private_contact_scan_states.is_empty();
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
