//! Per-wallet portion of [`ClientStartState`](crate::changeset::ClientStartState).
//!
//! Everything a single wallet contributes to the startup snapshot: the
//! key-wallet [`Wallet`] + [`ManagedWalletInfo`] pair, a lean
//! identity-manager snapshot, and still-unused asset locks bucketed by
//! account index.

use std::collections::BTreeMap;

use crate::changeset::identity_manager_start_state::IdentityManagerStartState;
use crate::wallet::asset_lock::tracked::TrackedAssetLock;
use dashcore::{OutPoint, Transaction};
use key_wallet::wallet::ManagedWalletInfo;
use key_wallet::Wallet;

/// Per-wallet slice of the startup snapshot.
///
/// Used as the value type in [`ClientStartState::wallets`](crate::changeset::ClientStartState::wallets).
#[derive(Debug)]
pub struct ClientWalletStartState {
    /// The key-wallet [`Wallet`] to rehydrate on startup. Carries the
    /// HD key material and account configuration the rest of the
    /// per-wallet state hangs off of.
    pub wallet: Wallet,
    /// Managed wallet info holding non-key-material state (balances,
    /// account metadata, UTXO set, etc.) for this wallet.
    pub wallet_info: ManagedWalletInfo,
    /// Lean snapshot of this wallet's
    /// [`IdentityManager`](crate::wallet::identity::IdentityManager):
    /// owned + watched identities, primary selection, and the
    /// gap-limit scan watermark.
    pub identity_manager: IdentityManagerStartState,
    /// Asset locks that have not yet been consumed by an identity
    /// registration / top-up, keyed by account index → outpoint.
    pub unused_asset_locks: BTreeMap<u32, BTreeMap<OutPoint, TrackedAssetLock>>,
    /// Outgoing transactions the host still has as unconfirmed, in
    /// `first_seen` order (a parent send precedes a child that spends
    /// its change).
    ///
    /// These are decoded here but deliberately NOT applied here: the
    /// spend has to travel through the normal
    /// `check_core_transaction(.., Mempool, ..)` path so `update_utxos`
    /// runs — dropping the input from `utxos` and recording it in
    /// `spent_outpoints`. That call is async and needs the `Wallet` and
    /// the `ManagedWalletInfo` together, so the replay happens at the
    /// async boundary in
    /// [`load_from_persistor`](crate::manager::load), not while this
    /// snapshot is being built.
    ///
    /// # Why the replay exists
    ///
    /// The spend effect of an unconfirmed outgoing transaction is never
    /// persisted: `isSpent` deliberately stays `false` on the input row
    /// until the spending transaction reaches a block, because a
    /// mempool-only sighting is reversible by eviction. A running app is
    /// still correct — the effect lives in memory. Across a restart it
    /// used to be recovered only by re-observing the transaction on the
    /// network, which is impossible for one that never got there: the
    /// input came back as spendable and the balance re-counted it,
    /// permanently. Replaying the record at load restores exactly the
    /// state the live process held.
    pub unconfirmed_outgoing_txs: Vec<Transaction>,
}
