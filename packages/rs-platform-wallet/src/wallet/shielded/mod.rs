//! Feature-gated shielded (Orchard / Halo 2) wallet support.
//!
//! This module provides ZK-private transactions on Dash Platform
//! using the Orchard circuit (Halo 2 proving system). It is
//! gated behind the `shielded` Cargo feature because it pulls in
//! heavy cryptographic dependencies.
//!
//! # Architecture
//!
//! - [`OrchardKeySet`] — ZIP-32 key derivation from a wallet seed.
//! - [`ShieldedStore`] / [`InMemoryShieldedStore`] — storage abstraction.
//!   The shared commitment tree lives here too; per-subwallet
//!   notes are scoped by [`SubwalletId`] inside the store.
//! - [`CachedOrchardProver`] — lazy-init proving key cache.
//! - [`ShieldedWallet`] — multi-account coordinator tying the
//!   wallet's Orchard accounts (`BTreeMap<u32, OrchardKeySet>`),
//!   the shared store, and the SDK together.

pub mod file_store;
pub mod keys;
pub mod note_selection;
pub mod operations;
pub mod prover;
pub mod store;
pub mod sync;

pub use file_store::{FileBackedShieldedStore, FileShieldedStoreError};
pub use keys::OrchardKeySet;
pub use prover::CachedOrchardProver;
pub use store::{InMemoryShieldedStore, ShieldedNote, ShieldedStore, SubwalletId};
pub use sync::{ShieldedSyncSummary, SyncNotesResult};

use std::collections::BTreeMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::changeset::ShieldedChangeSet;
use crate::changeset::{PlatformWalletChangeSet, ShieldedSyncStartState};
use crate::error::PlatformWalletError;
use crate::wallet::persister::WalletPersister;
use crate::wallet::platform_wallet::WalletId;

/// Per-account state held inside a [`ShieldedWallet`].
///
/// Crate-private — callers go through `ShieldedWallet`'s
/// per-account helpers (`default_address(account)`,
/// `balance(account)`, etc.). Held by value (not behind a lock)
/// because the parent wallet's `RwLock<S>` already serializes
/// access, and key material is read-only after derivation.
pub(super) struct AccountState {
    pub(super) keys: OrchardKeySet,
}

/// Feature-gated multi-account shielded wallet.
///
/// One [`ShieldedWallet`] lives inside one [`PlatformWallet`] and
/// holds every Orchard account that wallet has bound. Operations
/// take `account: u32` and route to the right keyset internally.
/// The shared `store: Arc<RwLock<S>>` is keyed per-account via
/// [`SubwalletId`] so multiple accounts on the same wallet (and
/// multiple wallets on the same network) cohabit the same store
/// without cross-talk.
pub struct ShieldedWallet<S: ShieldedStore> {
    /// Dash Platform SDK handle for network operations.
    pub(super) sdk: Arc<dash_sdk::Sdk>,
    /// 32-byte wallet identifier — used to construct
    /// [`SubwalletId`] for every store call.
    pub(super) wallet_id: WalletId,
    /// Bound Orchard accounts, keyed by ZIP-32 account index.
    pub(super) accounts: BTreeMap<u32, AccountState>,
    /// Pluggable storage backend behind a shared async lock. The
    /// commitment tree inside is global per network; notes are
    /// scoped per-subwallet by the store's `SubwalletId` keying.
    pub(super) store: Arc<RwLock<S>>,
    /// Optional persister handle. When set, every state-changing
    /// sync / spend pass emits a [`PlatformWalletChangeSet`] with
    /// a populated `shielded` field so the host (typically
    /// SwiftData on iOS) can mirror per-subwallet notes / sync
    /// watermarks. `None` means in-memory only — useful for
    /// tests and short-lived wallets.
    pub(super) persister: Option<WalletPersister>,
}

impl<S: ShieldedStore> ShieldedWallet<S> {
    /// Construct a [`ShieldedWallet`] from pre-derived keysets.
    ///
    /// `accounts` maps ZIP-32 account index → [`OrchardKeySet`].
    /// At least one account must be supplied.
    pub fn from_keysets(
        sdk: Arc<dash_sdk::Sdk>,
        wallet_id: WalletId,
        accounts: BTreeMap<u32, OrchardKeySet>,
        store: S,
    ) -> Result<Self, PlatformWalletError> {
        if accounts.is_empty() {
            return Err(PlatformWalletError::ShieldedKeyDerivation(
                "shielded wallet requires at least one account".to_string(),
            ));
        }
        let accounts = accounts
            .into_iter()
            .map(|(idx, keys)| (idx, AccountState { keys }))
            .collect();
        Ok(Self {
            sdk,
            wallet_id,
            accounts,
            store: Arc::new(RwLock::new(store)),
            persister: None,
        })
    }

    /// Attach a [`WalletPersister`] so future sync / spend passes
    /// emit shielded changesets to the host.
    pub fn set_persister(&mut self, persister: WalletPersister) {
        self.persister = Some(persister);
    }

    /// Queue a shielded changeset on the persister if one is
    /// attached. No-op otherwise.
    pub(super) fn queue_shielded_changeset(&self, cs: ShieldedChangeSet) {
        if cs.is_empty() {
            return;
        }
        let Some(persister) = &self.persister else {
            return;
        };
        let full = PlatformWalletChangeSet {
            shielded: Some(cs),
            ..Default::default()
        };
        if let Err(e) = persister.store(full) {
            tracing::warn!(
                wallet_id = %hex::encode(self.wallet_id),
                error = %e,
                "Failed to queue shielded changeset"
            );
        }
    }

    /// Rehydrate per-subwallet state from a persisted snapshot.
    /// Should be called after `from_seed_accounts(...)` and before
    /// the first sync pass so the in-memory store matches what
    /// the host already has on disk.
    pub async fn restore_from_snapshot(
        &self,
        snapshot: &ShieldedSyncStartState,
    ) -> Result<(), PlatformWalletError> {
        if snapshot.is_empty() {
            return Ok(());
        }
        let mut store = self.store.write().await;
        for (id, sub) in &snapshot.per_subwallet {
            // Only restore subwallets that belong to this wallet.
            if id.wallet_id != self.wallet_id {
                continue;
            }
            // Skip accounts that aren't bound on this wallet —
            // they'd accumulate state we can never spend.
            if !self.accounts.contains_key(&id.account_index) {
                continue;
            }
            for note in &sub.notes {
                store
                    .save_note(*id, note)
                    .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;
                if note.is_spent {
                    store
                        .mark_spent(*id, &note.nullifier)
                        .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;
                }
            }
            store
                .set_last_synced_note_index(*id, sub.last_synced_index)
                .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;
            if let Some((h, t)) = sub.nullifier_checkpoint {
                store
                    .set_nullifier_checkpoint(*id, h, t)
                    .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;
            }
        }
        Ok(())
    }

    /// Derive Orchard keys for every listed `account` from a
    /// wallet seed and return a [`ShieldedWallet`].
    ///
    /// `seed` is the BIP-39 seed bytes (32–252 bytes; typically
    /// 64). `network` selects the ZIP-32 coin type. Each entry of
    /// `accounts` becomes a separate ZIP-32 account
    /// (`m / 32' / coin_type' / account'`); duplicates are
    /// silently deduplicated.
    pub fn from_seed_accounts(
        sdk: Arc<dash_sdk::Sdk>,
        wallet_id: WalletId,
        seed: &[u8],
        network: dashcore::Network,
        accounts: &[u32],
        store: S,
    ) -> Result<Self, PlatformWalletError> {
        if accounts.is_empty() {
            return Err(PlatformWalletError::ShieldedKeyDerivation(
                "shielded wallet requires at least one account".to_string(),
            ));
        }
        let mut keysets: BTreeMap<u32, OrchardKeySet> = BTreeMap::new();
        for &account in accounts {
            let keys = OrchardKeySet::from_seed(seed, network, account)?;
            keysets.insert(account, keys);
        }
        Self::from_keysets(sdk, wallet_id, keysets, store)
    }

    /// Add another ZIP-32 account to this wallet by re-deriving
    /// from the seed. No-op if `account` is already bound.
    ///
    /// **Caveat**: the commitment tree only retains
    /// authentication paths for positions `Retention::Marked` at
    /// append time. Notes that reached the tree before this
    /// account existed were marked `Ephemeral` and can never
    /// produce witnesses for it without a tree wipe + full
    /// re-sync. New accounts therefore only see notes from
    /// future syncs. The host should drop the tree DB and
    /// re-sync from genesis when the user adds an account they
    /// expect to discover historical funds for.
    pub fn add_account_from_seed(
        &mut self,
        seed: &[u8],
        network: dashcore::Network,
        account: u32,
    ) -> Result<(), PlatformWalletError> {
        if self.accounts.contains_key(&account) {
            return Ok(());
        }
        let keys = OrchardKeySet::from_seed(seed, network, account)?;
        self.accounts.insert(account, AccountState { keys });
        Ok(())
    }

    /// All bound ZIP-32 account indices, in ascending order.
    pub fn account_indices(&self) -> Vec<u32> {
        self.accounts.keys().copied().collect()
    }

    /// `true` iff `account` is bound on this wallet.
    pub fn has_account(&self, account: u32) -> bool {
        self.accounts.contains_key(&account)
    }

    /// Borrow the keyset for `account`.
    pub(super) fn keys_for(&self, account: u32) -> Result<&OrchardKeySet, PlatformWalletError> {
        self.accounts.get(&account).map(|s| &s.keys).ok_or_else(|| {
            PlatformWalletError::ShieldedKeyDerivation(format!(
                "shielded account {account} not bound"
            ))
        })
    }

    /// Construct the [`SubwalletId`] for `account` on this wallet.
    pub(super) fn subwallet_id(&self, account: u32) -> SubwalletId {
        SubwalletId::new(self.wallet_id, account)
    }

    /// Total unspent shielded balance for `account` in credits.
    /// Reads from the store — does not trigger a sync.
    pub async fn balance(&self, account: u32) -> Result<u64, PlatformWalletError> {
        self.keys_for(account)?; // existence check
        let id = self.subwallet_id(account);
        let store = self.store.read().await;
        let notes = store
            .get_unspent_notes(id)
            .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;
        Ok(notes.iter().map(|n| n.value).sum())
    }

    /// Sum of unspent shielded balance across every bound account.
    pub async fn balance_total(&self) -> Result<u64, PlatformWalletError> {
        let store = self.store.read().await;
        let mut total: u64 = 0;
        for account in self.accounts.keys() {
            let id = SubwalletId::new(self.wallet_id, *account);
            let notes = store
                .get_unspent_notes(id)
                .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;
            total = total.saturating_add(notes.iter().map(|n| n.value).sum::<u64>());
        }
        Ok(total)
    }

    /// Per-account unspent shielded balance, in ascending account order.
    pub async fn balances(&self) -> Result<BTreeMap<u32, u64>, PlatformWalletError> {
        let store = self.store.read().await;
        let mut out: BTreeMap<u32, u64> = BTreeMap::new();
        for account in self.accounts.keys() {
            let id = SubwalletId::new(self.wallet_id, *account);
            let notes = store
                .get_unspent_notes(id)
                .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;
            out.insert(*account, notes.iter().map(|n| n.value).sum());
        }
        Ok(out)
    }

    /// The default payment address (diversifier index 0) for
    /// `account`. Returns an error if `account` isn't bound.
    pub fn default_address(
        &self,
        account: u32,
    ) -> Result<&grovedb_commitment_tree::PaymentAddress, PlatformWalletError> {
        self.keys_for(account).map(|k| &k.default_address)
    }

    /// Derive a payment address at `index` under `account`.
    pub fn address_at(
        &self,
        account: u32,
        index: u32,
    ) -> Result<grovedb_commitment_tree::PaymentAddress, PlatformWalletError> {
        Ok(self.keys_for(account)?.address_at(index))
    }
}
