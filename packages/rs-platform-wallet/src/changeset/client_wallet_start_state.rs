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
use key_wallet::{AccountType, Utxo, Wallet};

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
    /// Pending outgoing transactions, ordered with parents before children.
    /// Replay restores their engine records and accounting while persisted claims
    /// independently guard their inputs against funding redelivery.
    pub unconfirmed_outgoing_txs: Vec<Transaction>,
    /// Validated owned inputs temporarily staged for pending-send accounting replay.
    /// Only already-guarded inputs are staged; leftovers are removed before exposure.
    pub unconfirmed_outgoing_inputs: Vec<(AccountType, Utxo)>,
}
