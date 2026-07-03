//! Platform address wallet for DIP-17 platform payment addresses.

use std::collections::BTreeMap;
use std::sync::Arc;

use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use key_wallet::PlatformP2PKHAddress;
use tokio::sync::RwLock;

use crate::broadcaster::SpvBroadcaster;
use crate::error::PlatformWalletError;
use crate::wallet::asset_lock::manager::AssetLockManager;
use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
use key_wallet_manager::WalletManager;

use crate::wallet::persister::WalletPersister;

use super::provider::PlatformPaymentAddressProvider;

use dash_sdk::query_types::AddressInfos;

/// Platform address wallet providing DIP-17 platform payment address functionality.
#[derive(Clone)]
pub struct PlatformAddressWallet {
    pub(crate) sdk: Arc<dash_sdk::Sdk>,
    /// The shared wallet manager lock for all mutable wallet state.
    pub(crate) wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    /// Identifies which wallet within the manager this sub-wallet operates on.
    pub(crate) wallet_id: WalletId,
    /// Single provider covering every platform payment account on the
    /// wallet. `None` until [`initialize`] runs so that no-account
    /// wallets don't allocate empty state. Both sync and the
    /// post-broadcast reconciliation seam
    /// ([`reconcile_address_infos`](Self::reconcile_address_infos))
    /// take the `write` lock across their whole apply sequence, which
    /// is what serializes them against each other.
    pub(crate) provider: Arc<RwLock<Option<PlatformPaymentAddressProvider>>>,
    /// Shared asset-lock manager. Threaded in so the orchestrated
    /// `fund_from_asset_lock` path can drive
    /// build → IS-or-CL wait → consume on the same tracked locks
    /// every other sub-wallet sees. Cloned `Arc`, not owned.
    pub(crate) asset_locks: Arc<AssetLockManager<SpvBroadcaster>>,
    /// Per-wallet persistence handle for queuing changesets.
    pub(crate) persister: WalletPersister,
}

impl PlatformAddressWallet {
    /// Create a new PlatformAddressWallet without initializing the provider.
    ///
    /// Call [`initialize`] afterwards to build the unified provider.
    pub(crate) fn new(
        sdk: Arc<dash_sdk::Sdk>,
        wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        wallet_id: WalletId,
        asset_locks: Arc<AssetLockManager<SpvBroadcaster>>,
        persister: WalletPersister,
    ) -> Self {
        Self {
            sdk,
            wallet_manager,
            wallet_id,
            provider: Arc::new(RwLock::new(None)),
            asset_locks,
            persister,
        }
    }

    /// Build (or rebuild) the unified address provider covering every
    /// platform payment account on the wallet.
    ///
    /// Safe to call multiple times — later invocations re-scan the
    /// current account set from the wallet manager, picking up any
    /// accounts added since the last call. Sync state (watermark,
    /// `found`, `known_balances`) is **not** preserved across a
    /// rebuild; callers that need to preserve it should use
    /// [`restore_sync_state`] on the fresh provider.
    pub async fn initialize(&self) {
        match PlatformPaymentAddressProvider::from_wallets(
            Arc::clone(&self.wallet_manager),
            [self.wallet_id],
        )
        .await
        {
            Ok(provider) => {
                let mut guard = self.provider.write().await;
                *guard = Some(provider);
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to create platform address provider for wallet {}: {}",
                    hex::encode(self.wallet_id),
                    e
                );
            }
        }
    }

    /// Rebuild the provider from persisted state. Used on startup
    /// when a persister returned a non-empty
    /// [`PlatformAddressSyncStartState`](crate::PlatformAddressSyncStartState)
    /// — delegates to
    /// [`PlatformPaymentAddressProvider::from_persisted`] so xpubs,
    /// `found`, and `absent` are restored verbatim while `addresses`
    /// and `pending` are rebuilt from the live `AddressPool`.
    ///
    /// Also pushes each persisted balance back onto the matching
    /// `ManagedPlatformAccount` via `set_address_credit_balance` so
    /// the transfer/withdrawal `auto_select_inputs` paths see a
    /// non-zero balance immediately after restore — without this,
    /// they'd report "available 0 credits" until a fresh BLAST sync
    /// round fired `on_address_found` for every known address.
    /// Mirrors the `set_address_credit_balance(.., None)` shape in
    /// [apply.rs](crate::wallet::apply): `None` for the key-source
    /// argument because the gap-limit pool is already restored from
    /// `account_state.addresses` inside `from_persisted`.
    // TODO(CMT-004): no direct regression test for balance hydration via
    // initialize_from_persisted; future refactor could silently regress
    // restart visibility.
    pub async fn initialize_from_persisted(
        &self,
        persisted: crate::PlatformAddressSyncStartState,
    ) -> Result<(), PlatformWalletError> {
        // Hydrate `account.address_credit_balance` BEFORE constructing
        // the provider. `from_persisted` holds a read lock on
        // `wallet_manager` for its duration, and Tokio's `RwLock` has
        // no read→write upgrade — doing the write-lock dance first
        // keeps both paths simple and avoids exposing a new public
        // accessor on the provider.
        //
        // Required by spend paths that enumerate funded addresses
        // (e.g. `shielded_shield_from_account`): without this, after
        // a restart they read `available = 0` until the first BLAST
        // sync repopulates the in-memory map, even though SwiftData
        // reports a real balance to the UI.
        {
            let mut wm = self.wallet_manager.write().await;
            if let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) {
                for (account_index, account_state) in &persisted.per_account {
                    if let Some(account) = info
                        .core_wallet
                        .platform_payment_managed_account_at_index_mut(*account_index)
                    {
                        for (p2pkh, funds) in account_state.found() {
                            account.set_address_credit_balance(*p2pkh, funds.balance, None);
                        }
                    }
                }
            }
        }

        let mut per_wallet = std::collections::BTreeMap::new();
        per_wallet.insert(self.wallet_id, persisted.per_account);
        let provider = PlatformPaymentAddressProvider::from_persisted(
            Arc::clone(&self.wallet_manager),
            per_wallet,
            persisted.sync_height,
            persisted.sync_timestamp,
            persisted.last_known_recent_block,
        )
        .await?;
        let mut guard = self.provider.write().await;
        *guard = Some(provider);
        Ok(())
    }

    /// Reconcile platform-address balances from the proof-attested
    /// `address_infos` an address-funds state transition returns (transfer,
    /// withdrawal, asset-lock funding, identity registration / top-up /
    /// credit transfer). This is the single apply-and-persist seam every
    /// flow routes through.
    ///
    /// Each address's `(account_index, address_index)` is resolved through
    /// the provider's persisted `index <-> address` bijection — covering
    /// addresses restored from disk that are no longer in a live derived
    /// pool — with the live pools as fallback for addresses derived since
    /// the last sync (e.g. a fresh change address). Non-P2PKH addresses and
    /// addresses the wallet doesn't own (external recipients) are skipped.
    ///
    /// For each surviving entry the in-memory account balance is set to the
    /// proof's attested value, the provider's committed `found` seed is
    /// updated (so the background sync's diff baseline agrees with what we
    /// just applied), and a `PlatformAddressChangeSet` is persisted so the
    /// displayed balance and the next input selection both reflect on-chain
    /// reality. Without this, local balances stay frozen at their
    /// pre-transition values: the wallet displays a stale "Platform
    /// Balance" and the next input selection over-selects drained
    /// addresses, which Drive rejects with "Insufficient combined address
    /// balances".
    ///
    /// A freshness guard protects against racing the 15s background sync:
    /// entries whose nonce is below the committed seed's are dropped (a
    /// fresher state was already committed), see
    /// [`PlatformPaymentAddressProvider::commit_reconciliation`]. The
    /// provider write lock is held across the provider commit, the
    /// account-balance write, AND the persist — mirroring
    /// [`sync_balances`](Self::sync_balances), so the two writers' stores
    /// are totally ordered by the lock and a sync pass can never
    /// interleave between (or persist across) the seam's steps; the lock
    /// order (provider → wallet manager) matches the sync callbacks.
    ///
    /// Persistence errors are logged rather than propagated — Platform
    /// already accepted the transition, and a later sync reconciles.
    /// Callers that must observe persistence (e.g. asset-lock funding,
    /// which must not consume the lock over stale durable rows) use
    /// [`reconcile_address_infos_with_persistence`] instead.
    ///
    /// [`PlatformPaymentAddressProvider::commit_reconciliation`]:
    /// super::provider::PlatformPaymentAddressProvider::commit_reconciliation
    /// [`reconcile_address_infos_with_persistence`]:
    /// Self::reconcile_address_infos_with_persistence
    pub async fn reconcile_address_infos(
        &self,
        address_infos: &AddressInfos,
        context: &'static str,
    ) -> crate::PlatformAddressChangeSet {
        self.reconcile_address_infos_with_persistence(address_infos, context)
            .await
            .0
    }

    /// [`reconcile_address_infos`](Self::reconcile_address_infos), but
    /// also reporting whether the durable rows now reflect the
    /// transition: `true` when the changeset was stored (or nothing
    /// new needed storing), `false` when the balances could not be
    /// applied at all (no provider / no provider state / nothing
    /// resolved to a wallet-owned slot) or the persist itself failed.
    /// Callers with a persist-before-cleanup ordering invariant branch
    /// on the flag; everyone else uses the plain variant.
    pub(crate) async fn reconcile_address_infos_with_persistence(
        &self,
        address_infos: &AddressInfos,
        context: &'static str,
    ) -> (crate::PlatformAddressChangeSet, bool) {
        if address_infos.is_empty() {
            return (crate::PlatformAddressChangeSet::default(), true);
        }

        let mut guard = self.provider.write().await;
        let Some(provider) = guard.as_mut() else {
            tracing::warn!(
                wallet_id = ?self.wallet_id,
                context,
                "Address reconciliation skipped: no platform-address \
                 provider for this wallet; local balances stay stale \
                 until the next platform-address sync"
            );
            return (crate::PlatformAddressChangeSet::default(), false);
        };
        if provider.per_wallet_state(&self.wallet_id).is_none() {
            tracing::warn!(
                wallet_id = ?self.wallet_id,
                context,
                "Address reconciliation skipped: no platform-address \
                 provider state for this wallet; local balances stay \
                 stale until the next platform-address sync"
            );
            return (crate::PlatformAddressChangeSet::default(), false);
        }

        // Live-pool fallback indexes for addresses derived since the last
        // sync (not yet merged into the provider bijection). Taking the
        // wallet-manager read lock while holding the provider write lock
        // follows the provider → wallet-manager order the sync callbacks
        // use.
        let pool_indexes: BTreeMap<PlatformP2PKHAddress, (u32, u32)> = {
            let wm = self.wallet_manager.read().await;
            let mut out = BTreeMap::new();
            if let Some(info) = wm.get_wallet_info(&self.wallet_id) {
                for account in info.core_wallet.all_platform_payment_managed_accounts() {
                    // The provider tracks key-class-0 accounts only; other
                    // key classes have no per-account provider state to
                    // reconcile against.
                    if account.key_class != 0 {
                        continue;
                    }
                    for (&index, addr_info) in &account.addresses.addresses {
                        if let Ok(p2pkh) = PlatformP2PKHAddress::from_address(&addr_info.address) {
                            out.entry(p2pkh).or_insert((account.account, index));
                        }
                    }
                }
            }
            out
        };

        let outcome = provider.commit_reconciliation(&self.wallet_id, address_infos, &pool_indexes);

        if outcome.resolved == 0 {
            tracing::warn!(
                wallet_id = ?self.wallet_id,
                context,
                proof_addresses = address_infos.len(),
                "Address reconciliation resolved none of the proof's \
                 addresses to a wallet-owned slot. Expected when every \
                 address belongs to a third party; otherwise local \
                 balances stay stale until the next platform-address sync"
            );
            return (crate::PlatformAddressChangeSet::default(), false);
        }
        if outcome.stale_skipped > 0 || outcome.unchanged_skipped > 0 {
            tracing::debug!(
                wallet_id = ?self.wallet_id,
                context,
                stale_skipped = outcome.stale_skipped,
                unchanged_skipped = outcome.unchanged_skipped,
                "Address reconciliation dropped entries superseded by (or \
                 identical to) the committed sync seed"
            );
        }
        if outcome.entries.is_empty() {
            // Everything the proof attested was already committed (or
            // fresher) in the durable rows — nothing new to store.
            return (crate::PlatformAddressChangeSet::default(), true);
        }

        // Apply the proof-attested balances to the managed accounts while
        // STILL holding the provider write lock, so a background sync can't
        // interleave between the provider commit above and this write.
        // The per-account key source drives gap-limit extension when a
        // previously unfunded address (e.g. a change output) becomes funded.
        {
            let key_sources: BTreeMap<u32, key_wallet::KeySource> = outcome
                .entries
                .iter()
                .map(|e| e.account_index)
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .filter_map(|account_index| {
                    provider
                        .key_source(&self.wallet_id, account_index)
                        .map(|ks| (account_index, ks))
                })
                .collect();
            let mut wm = self.wallet_manager.write().await;
            if let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) {
                for entry in &outcome.entries {
                    if let Some(account) = info
                        .core_wallet
                        .platform_payment_managed_account_at_index_mut(entry.account_index)
                    {
                        account.set_address_credit_balance(
                            entry.address,
                            entry.funds.balance,
                            key_sources.get(&entry.account_index),
                        );
                    }
                }
            }
        }
        // Persist BEFORE releasing the provider lock, mirroring
        // `sync_balances`. Both writers persisting inside the same
        // critical section totally orders the stores with the lock: a
        // sync pass (or another reconciliation) that commits a fresher
        // seed after us also persists after us, so an older row can
        // never overwrite a fresher one on disk. Persistence callbacks
        // must not re-enter wallet APIs (the pre-existing contract —
        // stores already run under the wallet-manager write lock
        // elsewhere).
        let cs = crate::PlatformAddressChangeSet {
            addresses: outcome.entries,
            ..Default::default()
        };
        let mut persisted = true;
        if let Err(e) = self.persister.store(cs.clone().into()) {
            persisted = false;
            tracing::error!(
                context,
                error = %e,
                "Failed to persist platform-address reconciliation; \
                 in-memory balances are updated but durable rows stay stale \
                 until the next platform-address sync"
            );
        }
        drop(guard);
        (cs, persisted)
    }

    /// Get the network from the SDK.
    pub fn network(&self) -> key_wallet::Network {
        self.sdk.network
    }

    /// Wallet id this `PlatformAddressWallet` operates on. Exposed so
    /// FFI callers that build a `MnemonicResolverCoreSigner` on demand
    /// can thread the wallet id through to the resolver callback.
    /// Mirrors [`AssetLockManager::wallet_id`].
    pub fn wallet_id(&self) -> WalletId {
        self.wallet_id
    }

    /// Rebuild the provider so it covers a newly added account.
    ///
    /// Equivalent to [`initialize`]: the unified provider is rebuilt
    /// from the current account set in the wallet manager. The name
    /// is kept for API continuity with call sites that used to add
    /// per-account providers.
    pub async fn add_provider(&self, _account_index: u32) -> Result<(), PlatformWalletError> {
        self.initialize().await;
        Ok(())
    }

    /// Restore the incremental-sync watermark on the unified provider.
    ///
    /// Called during persisted-state replay so the next `sync_balances`
    /// call resumes from where the previous session left off instead of
    /// doing a full rescan. Zero-valued arguments are ignored (they mean
    /// "no stored watermark" — the provider keeps its fresh-start state).
    pub(crate) async fn apply_sync_state(
        &self,
        height: Option<u64>,
        timestamp: Option<u64>,
        last_known_recent_block: Option<u64>,
    ) {
        if height.is_none() && timestamp.is_none() && last_known_recent_block.is_none() {
            return;
        }
        let h = height.unwrap_or(0);
        let t = timestamp.unwrap_or(0);
        let r = last_known_recent_block.unwrap_or(0);
        let mut guard = self.provider.write().await;
        if let Some(provider) = guard.as_mut() {
            provider.set_stored_sync_state(h, t, r);
        }
    }

    /// Restore sync state from externally persisted values (e.g., SwiftData).
    ///
    /// Call this after `initialize()` and before the first sync to resume
    /// incremental mode instead of doing a full trunk/branch/compact rescan.
    pub async fn restore_sync_state(
        &self,
        sync_height: u64,
        sync_timestamp: u64,
        last_known_recent_block: u64,
    ) {
        self.apply_sync_state(
            Some(sync_height),
            Some(sync_timestamp),
            Some(last_known_recent_block),
        )
        .await;
    }

    /// Reset the platform-address sync watermark and drop every cached
    /// balance for this wallet, forcing a full trunk/branch/compact
    /// rescan on the next `sync_balances`.
    ///
    /// Backs the host's "Clear" flow. Clears BOTH in-memory balance
    /// stores a resume would otherwise read from:
    ///   * the provider's incremental seed (`found`) + watermark — what
    ///     makes a resync "fast" (see
    ///     [`PlatformPaymentAddressProvider::reset_sync_state`]);
    ///   * each `ManagedPlatformAccount`'s `address_balances` map — what
    ///     [`addresses_with_balances`](Self::addresses_with_balances) /
    ///     `total_credits` and the transfer/withdraw spend paths read.
    ///     Without this the UI/spend paths would keep reporting stale
    ///     balances until the next full sync re-zeroed them via the
    ///     absent diff.
    ///
    /// Does NOT route through [`apply_sync_state`] — that helper's
    /// all-None early-return guard is meant for persisted-state replay
    /// and is irrelevant here. Both clears run inside one provider
    /// write critical section (provider → wallet-manager, the same
    /// nesting order [`sync_balances`](Self::sync_balances) and the
    /// reconciliation seam use), so a concurrent sync/reconciliation
    /// pass can never interleave between the two steps and repopulate
    /// the managed balances after only one store was cleared.
    pub async fn reset_sync_state(&self) {
        let mut guard = self.provider.write().await;
        if let Some(provider) = guard.as_mut() {
            provider.reset_sync_state();
        }
        let mut wm = self.wallet_manager.write().await;
        if let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) {
            for account in info.core_wallet.all_platform_payment_managed_accounts_mut() {
                account.clear_balances();
            }
        }
    }

    /// Internal accessor for the diagnostic snapshot path on
    /// [`crate::manager::PlatformWalletManager`]. The provider lock is
    /// otherwise crate-private — the manager-level snapshot needs to
    /// `blocking_read` it, which requires re-exposing the `Arc`.
    pub(crate) fn provider_for_diagnostics(
        &self,
    ) -> Arc<RwLock<Option<super::provider::PlatformPaymentAddressProvider>>> {
        Arc::clone(&self.provider)
    }
}

impl PlatformAddressWallet {
    /// Get the next unused platform payment receive address from the
    /// HD address pool for the given account key. Generates a new
    /// address if the pool is exhausted, maintaining the gap limit.
    ///
    /// DIP-17 derivation: `m/9'/coin_type'/17'/account'/key_class'/index`
    /// - `account_key.account` selects the HD account
    /// - `account_key.key_class` selects the key purpose (0 = clear funds)
    ///
    /// The address is derived from the wallet's public key material
    /// via dashcore's `AddressPool::next_unused` — no seed access or
    /// caller-side derivation needed.
    pub async fn next_unused_receive_address(
        &self,
        account_key: key_wallet::account::account_collection::PlatformPaymentAccountKey,
    ) -> Result<PlatformAddress, PlatformWalletError> {
        let mut wm = self.wallet_manager.write().await;
        let (wallet, info) = wm
            .get_wallet_mut_and_info_mut(&self.wallet_id)
            .ok_or_else(|| {
                PlatformWalletError::WalletNotFound(format!(
                    "Wallet {:?} not found",
                    hex::encode(self.wallet_id)
                ))
            })?;

        let managed_account = info
            .core_wallet
            .platform_payment_managed_account_at_index_mut(account_key.account)
            .ok_or_else(|| {
                PlatformWalletError::AddressSync(format!(
                    "No platform payment account at index {}",
                    account_key.account
                ))
            })?;

        let key_source = {
            let xpub = wallet
                .accounts
                .platform_payment_accounts
                .get(&account_key)
                .map(|acct| acct.account_xpub)
                .ok_or_else(|| {
                    PlatformWalletError::AddressSync(format!(
                        "No platform payment account key for {:?}",
                        account_key
                    ))
                })?;
            key_wallet::KeySource::Public(xpub)
        };

        let address = managed_account
            .addresses
            .next_unused(&key_source, true)
            .map_err(|e| PlatformWalletError::AddressSync(e.to_string()))?;

        PlatformAddress::try_from(address).map_err(|e| {
            PlatformWalletError::AddressSync(format!("Failed to convert to PlatformAddress: {e}"))
        })
    }

    /// Get all platform addresses with their cached balances.
    ///
    /// Returns the balances from the last call to [`sync_balances`](Self::sync_balances),
    /// [`transfer`](Self::transfer), or [`withdraw`](Self::withdraw).
    pub async fn addresses_with_balances(&self) -> Vec<(PlatformAddress, Credits)> {
        let wm = self.wallet_manager.read().await;
        wm.get_wallet_info(&self.wallet_id)
            .and_then(|info| info.core_wallet.first_platform_payment_managed_account())
            .map(|account| {
                account
                    .address_balances
                    .iter()
                    .map(|(p2pkh, &bal)| (PlatformAddress::P2pkh(p2pkh.to_bytes()), bal))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Current incremental-sync watermark (`last_known_recent_block`)
    /// from the unified platform-address provider.
    ///
    /// Returns `None` when the provider hasn't been initialised yet or
    /// when no incremental sync has produced a watermark. A zero-valued
    /// watermark is reported as `None` to match the "no stored watermark"
    /// convention used by [`Self::apply_sync_state`]. Intended for
    /// progress checks where the precise "uninitialised vs. zero"
    /// distinction is not material.
    pub async fn sync_watermark(&self) -> Option<u64> {
        let guard = self.provider.read().await;
        let raw = guard.as_ref().map(|p| p.last_known_recent_block())?;
        (raw > 0).then_some(raw)
    }

    /// Get total platform credits across all addresses.
    ///
    /// Returns the sum of all cached balances.
    pub async fn total_credits(&self) -> Credits {
        let wm = self.wallet_manager.read().await;
        wm.get_wallet_info(&self.wallet_id)
            .and_then(|info| info.core_wallet.first_platform_payment_managed_account())
            .map(|account| account.total_credit_balance())
            .unwrap_or(0)
    }
}

impl std::fmt::Debug for PlatformAddressWallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlatformAddressWallet")
            .field("network", &self.sdk.network)
            .finish()
    }
}
