//! Lean startup snapshot for [`IdentityManager`](crate::wallet::identity::IdentityManager).
//!
//! Mirrors the persistable buckets of `IdentityManager` as a plain data
//! struct — no methods, no invariants, no live handles — so persisters
//! can round-trip it without dragging in the manager's business logic.

use std::collections::BTreeMap;

use dpp::prelude::Identifier;

use crate::changeset::IdentityScanStateEntry;
use crate::wallet::identity::ManagedIdentity;
use crate::wallet::identity::RegistrationIndex;
use crate::wallet::platform_wallet::WalletId;

/// Restored [`IdentityManager`](crate::wallet::identity::IdentityManager)
/// state carried in [`ClientWalletStartState`](crate::changeset::ClientWalletStartState).
///
/// Two-bucket shape — see
/// [`IdentityManager`](crate::wallet::identity::IdentityManager) for
/// the layout rationale.
#[derive(Debug, Default)]
pub struct IdentityManagerStartState {
    /// Observed identities the client doesn't own keys for, keyed by
    /// identity id.
    pub out_of_wallet_identities: BTreeMap<Identifier, ManagedIdentity>,
    /// Wallet-owned identities, outer-keyed by wallet id and
    /// inner-keyed by BIP-9 registration index.
    pub wallet_identities: BTreeMap<WalletId, BTreeMap<RegistrationIndex, ManagedIdentity>>,
    /// Per-wallet verdict of the last gap-limit identity scan.
    ///
    /// An absent entry means "no verdict was restored", which is NOT the same
    /// as a complete scan — a host that does not persist the verdict yet, and
    /// a wallet whose first scan has not run, both land here. Absence
    /// therefore preserves the existing warm-launch behaviour rather than
    /// claiming a guarantee nobody made; only a restored `complete: false`
    /// forces a rescan. See [`IdentityScanStateEntry`].
    pub scan_states: BTreeMap<WalletId, IdentityScanStateEntry>,
}

impl IdentityManagerStartState {
    /// Drop the private contact request scan state every restored identity
    /// carries (its read position and its contacts' shielded addresses), as a
    /// load does for a backend that does not attest
    /// [`PRIVATE_CONTACT_REQUEST_SCAN_STATE`](crate::changeset::PersistenceCapabilities::PRIVATE_CONTACT_REQUEST_SCAN_STATE):
    /// nothing keeps such a backend's copy current, so the store is read from
    /// its start.
    pub(crate) fn forget_private_contact_scan_state(&mut self) {
        for managed in self
            .wallet_identities
            .values_mut()
            .flat_map(BTreeMap::values_mut)
            .chain(self.out_of_wallet_identities.values_mut())
        {
            managed.forget_private_contact_scan_state();
        }
    }
}
