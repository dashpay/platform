//! Lean startup snapshot for [`IdentityManager`](crate::wallet::identity::IdentityManager).
//!
//! Mirrors the persistable buckets of `IdentityManager` as a plain data
//! struct — no methods, no invariants, no live handles — so persisters
//! can round-trip it without dragging in the manager's business logic.

use std::collections::{BTreeMap, BTreeSet};

use dpp::prelude::Identifier;

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
    /// Wallet ids whose most recent identity-discovery scan answered every
    /// gap-limit probe (zero failed probes) — see
    /// [`IdentityManager::is_wallet_fully_discovered`](crate::wallet::identity::IdentityManager::is_wallet_fully_discovered).
    /// A backend that persists the `identity_discovery_complete` changeset flag
    /// populates this so a genuinely-complete wallet keeps its startup
    /// warm-launch shortcut across restart; a backend that does not leaves it
    /// empty, which is the safe default (re-scan rather than strand, #4365).
    pub fully_discovered_wallets: BTreeSet<WalletId>,
}
