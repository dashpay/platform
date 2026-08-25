//! Platform address wallet for DIP-17 platform payment addresses.

use std::collections::{BTreeMap, BTreeSet};
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

/// Merge transient derived addresses with persisted, hydrated balance keys.
///
/// A `BTreeSet` deduplicates addresses present in both sources and gives every
/// payment-address operation the same deterministic post-relaunch candidate
/// set.
pub(crate) fn merge_platform_payment_candidate_addresses(
    derived_addresses: impl IntoIterator<Item = PlatformP2PKHAddress>,
    hydrated_addresses: impl IntoIterator<Item = PlatformP2PKHAddress>,
) -> BTreeSet<PlatformP2PKHAddress> {
    derived_addresses
        .into_iter()
        .chain(hydrated_addresses)
        .collect()
}

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

    /// Enumerate the candidate address SET for `account_index`: the union of
    /// the account's transient derived pool (`addresses.addresses`) and the
    /// hydrated `address_balances` map.
    ///
    /// The union is what lets input selection survive a fresh relaunch — the
    /// derived pool is empty until a platform sync repopulates it, while
    /// `address_balances` is hydrated synchronously on wallet load from the
    /// persisted `platform_addresses` rows (the same source Platform Balance
    /// reads). Enumerating only the pool made selection find no candidates and
    /// fail right after launch even though the balances were on disk and
    /// on-chain.
    ///
    /// Balances are NOT read here — every caller re-reads the authoritative
    /// on-chain balance via `AddressInfo::fetch_many`, keeping the submit gate
    /// and the spend path in lockstep and immune to a stale/zero cache. The
    /// wallet-manager read lock is released before returning so a concurrent
    /// sync/reconcile is never blocked behind a caller's proof round-trip.
    pub(crate) async fn candidate_address_set(
        &self,
        account_index: u32,
    ) -> Result<BTreeSet<PlatformAddress>, PlatformWalletError> {
        let wm = self.wallet_manager.read().await;
        let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
            PlatformWalletError::WalletNotFound(format!(
                "Wallet {:?} not found in wallet manager",
                hex::encode(self.wallet_id)
            ))
        })?;

        let account = info
            .core_wallet
            .platform_payment_managed_account_at_index(account_index)
            .ok_or_else(|| {
                PlatformWalletError::AddressSync(format!(
                    "No platform payment account at index {}",
                    account_index
                ))
            })?;

        Ok(merge_platform_payment_candidate_addresses(
            account
                .addresses
                .addresses
                .values()
                .filter_map(|addr_info| {
                    PlatformP2PKHAddress::from_address(&addr_info.address).ok()
                }),
            account.address_balances.keys().copied(),
        )
        .into_iter()
        .map(|p2pkh| PlatformAddress::P2pkh(p2pkh.to_bytes()))
        .collect())
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
    /// height-pin authority (see `AddressFunds::as_of_height`) — entries
    /// whose pin is below the committed seed's are dropped (a fresher
    /// absolute was already committed; the nonce breaks same-block ties),
    /// see [`PlatformPaymentAddressProvider::commit_reconciliation`]. The
    /// provider write lock is held across the provider commit, the
    /// account-balance write, AND the persist — mirroring
    /// [`sync_balances`](Self::sync_balances), so the two writers' stores
    /// are totally ordered by the lock and a sync pass can never
    /// interleave between (or persist across) the seam's steps; the lock
    /// order (provider → wallet manager) matches the sync callbacks.
    ///
    /// # `as_of_height` — the height pin
    ///
    /// `as_of_height` is the block height of the proof that attested
    /// `address_infos` (the broadcast result's quorum-authenticated
    /// `metadata.height`). Every committed entry carries it as its
    /// balance pin, which is what makes the optimistic absolute write
    /// safe against delta replay: the transition's on-chain
    /// `AddBalanceToAddress` credit is recorded as a DELTA
    /// (`AddToCredits`) at this same height in Drive's
    /// recent-address-balance-changes tree, and the sync's apply loops
    /// drop any delta at or below an entry's pin. This replaces the
    /// former watermark-invalidation gate (`credited_outputs`), which
    /// forced a full rescan but could not stop the rescan itself from
    /// replaying the same delta on top of the fresh absolute.
    ///
    /// Persistence errors are logged rather than propagated — Platform
    /// already accepted the transition, and a later sync reconciles.
    ///
    /// [`PlatformPaymentAddressProvider::commit_reconciliation`]:
    /// super::provider::PlatformPaymentAddressProvider::commit_reconciliation
    pub async fn reconcile_address_infos(
        &self,
        address_infos: &AddressInfos,
        as_of_height: u64,
        context: &'static str,
    ) -> crate::PlatformAddressChangeSet {
        self.reconcile_address_infos_with_persistence(address_infos, as_of_height, context)
            .await
            .0
    }

    /// Like [`Self::reconcile_address_infos`], but also reports whether the
    /// reconciled balance changeset was **durably persisted**.
    ///
    /// `persisted == false` means (and only means) that the in-memory
    /// managed-account balances WERE updated to the proof-attested values
    /// but the durable `persister.store(...)` write failed — so a restart
    /// would reseed from the stale rows. Every early-return path (no
    /// provider, nothing resolved, nothing changed) leaves memory
    /// untouched, so there is no memory-vs-disk divergence and
    /// `persisted` is `true` there. Callers that pair reconciliation with
    /// a one-shot side effect — notably `fund_from_asset_lock` marking an
    /// asset lock `Consumed` — must gate that side effect on
    /// `persisted == true`, or they risk pairing an irreversible
    /// commitment with durable balances that under-report the spend.
    pub(super) async fn reconcile_address_infos_with_persistence(
        &self,
        address_infos: &AddressInfos,
        as_of_height: u64,
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
            return (crate::PlatformAddressChangeSet::default(), true);
        };
        if provider.per_wallet_state(&self.wallet_id).is_none() {
            tracing::warn!(
                wallet_id = ?self.wallet_id,
                context,
                "Address reconciliation skipped: no platform-address \
                 provider state for this wallet; local balances stay \
                 stale until the next platform-address sync"
            );
            return (crate::PlatformAddressChangeSet::default(), true);
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

        let outcome = provider.commit_reconciliation(
            &self.wallet_id,
            address_infos,
            &pool_indexes,
            as_of_height,
        );

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
            return (crate::PlatformAddressChangeSet::default(), true);
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

        let persisted = match self.persister.store(cs.clone().into()) {
            Ok(()) => true,
            Err(e) => {
                tracing::error!(
                    context,
                    error = %e,
                    "Failed to persist platform-address reconciliation; \
                     in-memory balances are updated but durable rows stay stale \
                     until the next platform-address sync"
                );
                false
            }
        };
        drop(guard);
        (cs, persisted)
    }

    /// Get the network from the SDK.
    pub fn network(&self) -> key_wallet::Network {
        self.sdk.network
    }

    /// The per-input minimum credit amount enforced by the chain for
    /// address-funds transitions, read from the wallet's **current**
    /// platform version
    /// (`platform_version.dpp.state_transitions.address_funds.min_input_amount`).
    ///
    /// This is the same constant the transfer/withdraw auto-selectors use
    /// to drop sub-minimum "dust" inputs (see
    /// [`select_withdrawable_inputs`](super::withdrawal) and
    /// [`build_auto_select_candidates`](super::transfer)): DPP rejects any
    /// address-funds input below this floor, so an address whose balance is
    /// under it cannot be spent on its own. Exposed so UI gating can sum
    /// only spendable (≥ this) balances instead of every funded row,
    /// keeping the enabled/disabled decision in step with what the Rust
    /// selectors will actually consume.
    ///
    /// The version is resolved from the wallet's SDK
    /// ([`dash_sdk::Sdk::version`]), the same network-floored,
    /// protocol-version-tracking source the spend paths run under — so the
    /// figure is version-locked rather than a hardcoded mirror of the
    /// constant.
    pub fn min_input_amount(&self) -> Credits {
        self.sdk
            .version()
            .dpp
            .state_transitions
            .address_funds
            .min_input_amount
    }

    /// The per-output minimum credit amount enforced by the chain for
    /// address-funds transitions, read from the wallet's **current**
    /// platform version
    /// (`platform_version.dpp.state_transitions.address_funds.min_output_amount`).
    ///
    /// DPP rejects any address-funds *output* below this floor, so a transfer
    /// that sends a single output under it deterministically fails structure
    /// validation after submit. Exposed so UI gating can disable submit (and
    /// explain why) when the requested amount is below the minimum, keeping
    /// the enabled/disabled decision in step with what DPP will accept —
    /// rather than mirroring the protocol constant in Swift, which would
    /// drift if the version changed it.
    ///
    /// The version is resolved from the wallet's SDK
    /// ([`dash_sdk::Sdk::version`]), the same network-floored,
    /// protocol-version-tracking source the spend paths run under, so the
    /// figure is version-locked. Companion to [`min_input_amount`](Self::min_input_amount).
    pub fn min_output_amount(&self) -> Credits {
        self.sdk
            .version()
            .dpp
            .state_transitions
            .address_funds
            .min_output_amount
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
    /// and is irrelevant here. The wallet-manager write is nested
    /// INSIDE the provider write (provider → wallet-manager, the same
    /// order `reconcile_address_infos` uses) so the two clears are
    /// atomic with respect to a concurrent sync or reconciliation.
    pub async fn reset_sync_state(&self) {
        // Hold the provider write lock across BOTH the managed-account
        // balance clear AND the provider reset, so the whole reset is
        // atomic: without it, a sync pass or a reconciliation could
        // interleave between the two steps and repopulate the managed
        // balances (or the provider seed) we just cleared, leaving the
        // "Clear" half-applied. The wallet-manager write is nested
        // INSIDE the provider write — the same provider → wallet-manager
        // lock order `reconcile_address_infos` uses — so the two paths
        // can't deadlock.
        let mut guard = self.provider.write().await;
        {
            let mut wm = self.wallet_manager.write().await;
            if let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) {
                for account in info.core_wallet.all_platform_payment_managed_accounts_mut() {
                    account.clear_balances();
                }
            }
        }
        if let Some(provider) = guard.as_mut() {
            provider.reset_sync_state();
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
    /// The address is atomically reserved in the pool before it is
    /// returned. Observing funds later promotes it from reserved to used.
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
            .next_unused_and_reserve(&key_source, crate::util::now_secs())
            .map_err(|e| PlatformWalletError::AddressSync(e.to_string()))?;

        PlatformAddress::try_from(address).map_err(|e| {
            PlatformWalletError::AddressSync(format!("Failed to convert to PlatformAddress: {e}"))
        })
    }

    /// Release a receive-address reservation back to the available pool.
    ///
    /// Returns `false` when the address is non-P2PKH, unknown to the selected
    /// account, already released, or no longer reserved because it was used.
    ///
    /// # Errors
    ///
    /// Returns an error when the wallet or selected platform-payment account
    /// is not registered with the wallet manager.
    pub async fn release_receive_reservation(
        &self,
        account_key: key_wallet::account::account_collection::PlatformPaymentAccountKey,
        address: &PlatformAddress,
    ) -> Result<bool, PlatformWalletError> {
        let PlatformAddress::P2pkh(hash) = address else {
            return Ok(false);
        };

        let mut wm = self.wallet_manager.write().await;
        let info = wm.get_wallet_info_mut(&self.wallet_id).ok_or_else(|| {
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

        let dash_address =
            PlatformP2PKHAddress::new(*hash).to_address(managed_account.addresses.network);
        let Some(index) = managed_account.addresses.address_index(&dash_address) else {
            return Ok(false);
        };

        Ok(managed_account.addresses.release_reservation(index))
    }

    /// Get all platform addresses with their cached balances.
    ///
    /// Returns the balances from the last call to [`sync_balances`](Self::sync_balances),
    /// [`transfer`](Self::transfer), or [`withdraw`](Self::withdraw).
    ///
    /// Resolves against the **first** platform-payment account (account index 0,
    /// key class 0). This is a read-only display query; account-scoped input
    /// selection for transfers/withdrawals happens inside
    /// [`transfer`](Self::transfer) / [`withdraw`](Self::withdraw) via
    /// [`InputSelection::Auto`](super::InputSelection::Auto), which resolves the
    /// requested account on the Rust side.
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
    /// convention used by [`Self::apply_sync_state`]. The value is
    /// monotonic non-decreasing across syncs against the same chain — a
    /// later sync can only advance the watermark, never roll it back.
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

    /// Highest derived index in the platform-payment receive pool for
    /// the given account, combining the synced-balance map and the
    /// eager `highest_generated` watermark. `None` when neither side
    /// has produced an index (no syncs yet **and** the pool was built
    /// with `gap_limit == 0`, which doesn't occur in production).
    ///
    /// Used by test infrastructure (e2e sweep / funding paths) to size
    /// the `SimpleSigner` key window — the signer must cover every
    /// index the pool may hand to a `transfer` input selector. Production
    /// transfer/withdraw paths use the modern provider and don't call
    /// this accessor.
    ///
    /// TODO: this currently reads from the deprecated
    /// `platform_payment_managed_account.addresses` pool. Migrate to
    /// `PlatformPaymentAddressProvider` once it exposes a stateful
    /// pool (per @QuantumExplorer's review on #3648). Callers don't
    /// change — the accessor's implementation flips.
    pub async fn platform_payment_account_max_derived_index(
        &self,
        account_index: u32,
    ) -> Result<Option<u32>, PlatformWalletError> {
        let wm = self.wallet_manager.read().await;
        let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
            PlatformWalletError::WalletNotFound(format!(
                "Wallet {:?} not found",
                hex::encode(self.wallet_id)
            ))
        })?;
        let account = info
            .core_wallet
            .platform_payment_managed_account_at_index(account_index)
            .ok_or_else(|| {
                PlatformWalletError::AddressSync(format!(
                    "No platform payment account at index {account_index}"
                ))
            })?;
        let synced_max = account.addresses.addresses.keys().copied().max();
        let generated_max = account.addresses.highest_generated;
        Ok(synced_max.into_iter().chain(generated_max).max())
    }

    /// Returns the configured `gap_limit` on the platform-payment receive
    /// pool for the given account.
    ///
    /// TODO: this currently reads from the deprecated
    /// `platform_payment_managed_account.addresses` pool. Migrate to
    /// `PlatformPaymentAddressProvider` once it exposes a stateful
    /// pool (per @QuantumExplorer's review on #3648).
    pub async fn platform_payment_account_gap_limit(
        &self,
        account_index: u32,
    ) -> Result<u32, PlatformWalletError> {
        let wm = self.wallet_manager.read().await;
        let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
            PlatformWalletError::WalletNotFound(format!(
                "Wallet {:?} not found",
                hex::encode(self.wallet_id)
            ))
        })?;
        let account = info
            .core_wallet
            .platform_payment_managed_account_at_index(account_index)
            .ok_or_else(|| {
                PlatformWalletError::AddressSync(format!(
                    "No platform payment account at index {account_index}"
                ))
            })?;
        Ok(account.addresses.gap_limit)
    }
}

impl PlatformAddressWallet {
    /// Force-seed the cached credit balance for a platform payment address.
    ///
    /// Called by the e2e harness after a dual-verified `AddressInfo::fetch`
    /// confirms the on-chain balance when the BLAST sync path consistently
    /// returns the address as NOT FOUND (DAPI replica divergence, issue #3611).
    ///
    /// Mirrors what `on_address_found` does during a successful BLAST sync
    /// (`provider.rs:621`) and what `fund_from_asset_lock` does after on-chain
    /// confirmation (`fund_from_asset_lock.rs:429`) — both call
    /// `account.set_address_credit_balance` directly.
    ///
    /// Only call this after proof-verified dual confirmation. Nonce is not
    /// injected because `transfer()` fetches the current nonce from DAPI at
    /// broadcast time (matching the `apply_changeset` precedent at `apply.rs:272`).
    pub async fn inject_address_balance(
        &self,
        account_index: u32,
        address: PlatformP2PKHAddress,
        balance: Credits,
    ) -> Result<(), PlatformWalletError> {
        let mut wm = self.wallet_manager.write().await;
        let info = wm.get_wallet_info_mut(&self.wallet_id).ok_or_else(|| {
            PlatformWalletError::WalletNotFound(format!(
                "wallet {} not in wallet manager",
                hex::encode(self.wallet_id),
            ))
        })?;
        if let Some(account) = info
            .core_wallet
            .platform_payment_managed_account_at_index_mut(account_index)
        {
            account.set_address_credit_balance(address, balance, None);
            tracing::info!(
                balance,
                %address,
                account_index,
                "inject_address_balance: spend cache seeded with verified balance"
            );
        }
        Ok(())
    }
}

impl std::fmt::Debug for PlatformAddressWallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlatformAddressWallet")
            .field("network", &self.sdk.network)
            .finish()
    }
}

#[cfg(test)]
mod found_026_tests {
    use super::*;
    use crate::wallet::persister::{NoPlatformPersistence, WalletPersister};
    use key_wallet::account::account_collection::PlatformPaymentAccountKey;
    use key_wallet::wallet::initialization::{
        PlatformPaymentAccountSpec, WalletAccountCreationOptions,
    };
    use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
    use key_wallet::{Network, Wallet};
    use key_wallet_manager::WalletManager;
    use std::collections::{BTreeMap, BTreeSet};

    const ACCOUNT_KEY: PlatformPaymentAccountKey = PlatformPaymentAccountKey {
        account: 0,
        key_class: 0,
    };

    /// Build a network-free `PlatformAddressWallet` over one DIP-17
    /// platform-payment account (account 0, key_class 0). Mirrors the
    /// `register_wallet` path: `ManagedWalletInfo::from_wallet` +
    /// `insert_wallet`, no SPV / no funding.
    fn wallet_with_platform_account() -> PlatformAddressWallet {
        use crate::events::PlatformEventManager;
        use crate::spv::SpvRuntime;
        use crate::wallet::asset_lock::manager::AssetLockManager;
        use tokio::sync::Notify;

        let mut pp = BTreeSet::new();
        pp.insert(PlatformPaymentAccountSpec {
            account: 0,
            key_class: 0,
        });
        let opts = WalletAccountCreationOptions::AllAccounts(
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::new(),
            pp,
        );
        let wallet = Wallet::new_random(Network::Testnet, opts).expect("wallet");

        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let info = PlatformWalletInfo {
            core_wallet: ManagedWalletInfo::from_wallet(&wallet, 0),
            generation: Arc::new(crate::wallet::core::WalletGeneration::new()),
            identity_manager: crate::wallet::identity::IdentityManager::new(),
            tracked_asset_locks: BTreeMap::new(),
            dpns_name_states: BTreeMap::new(),
        };
        let wallet_manager = Arc::new(RwLock::new(WalletManager::new(Network::Testnet)));
        let wallet_id = wallet_manager
            .try_write()
            .expect("uncontended")
            .insert_wallet(wallet, info)
            .expect("insert");
        let persister = WalletPersister::new(wallet_id, Arc::new(NoPlatformPersistence));
        let event_manager = Arc::new(PlatformEventManager::new(Vec::new()));
        let spv = Arc::new(SpvRuntime::new(Arc::clone(&wallet_manager), event_manager));
        let broadcaster = Arc::new(SpvBroadcaster::new(spv));
        let asset_locks = Arc::new(AssetLockManager::new(
            Arc::clone(&sdk),
            Arc::clone(&wallet_manager),
            wallet_id,
            Arc::new(Notify::new()),
            broadcaster,
            persister.clone(),
        ));
        PlatformAddressWallet::new(sdk, wallet_manager, wallet_id, asset_locks, persister)
    }

    /// Found-026 durable guard: two `next_unused_receive_address` calls
    /// with NO intervening sync/balance update must return DISTINCT
    /// addresses. The first call reserves index 0 without marking it
    /// used, so the second yields index 1.
    #[tokio::test]
    async fn found_026_back_to_back_handout_returns_distinct_addresses() {
        let wallet = wallet_with_platform_account();

        let a = wallet
            .next_unused_receive_address(ACCOUNT_KEY)
            .await
            .expect("first hand-out");
        let b = wallet
            .next_unused_receive_address(ACCOUNT_KEY)
            .await
            .expect("second hand-out");

        assert_ne!(
            a, b,
            "back-to-back hand-out with no sync re-handed the same address (Found-026)"
        );
    }

    #[tokio::test]
    async fn release_receive_reservation_is_idempotent_and_miss_safe() {
        let wallet = wallet_with_platform_account();
        let unknown = PlatformAddress::P2pkh([0xff; 20]);
        let non_p2pkh = PlatformAddress::P2sh([0xee; 20]);

        assert!(!wallet
            .release_receive_reservation(ACCOUNT_KEY, &unknown)
            .await
            .expect("unknown address release"));
        assert!(!wallet
            .release_receive_reservation(ACCOUNT_KEY, &non_p2pkh)
            .await
            .expect("non-P2PKH release"));

        let reserved = wallet
            .next_unused_receive_address(ACCOUNT_KEY)
            .await
            .expect("reserve");
        assert!(wallet
            .release_receive_reservation(ACCOUNT_KEY, &reserved)
            .await
            .expect("first release"));
        assert!(!wallet
            .release_receive_reservation(ACCOUNT_KEY, &reserved)
            .await
            .expect("second release"));

        let reissued = wallet
            .next_unused_receive_address(ACCOUNT_KEY)
            .await
            .expect("reissue");
        assert_eq!(reissued, reserved);

        {
            let mut wm = wallet.wallet_manager.write().await;
            let (_, info) = wm
                .get_wallet_mut_and_info_mut(&wallet.wallet_id)
                .expect("wallet present");
            let pool = &mut info
                .core_wallet
                .platform_payment_managed_account_at_index_mut(ACCOUNT_KEY.account)
                .expect("managed account")
                .addresses;
            assert!(pool.mark_index_used(0));
        }
        assert!(!wallet
            .release_receive_reservation(ACCOUNT_KEY, &reserved)
            .await
            .expect("used address release"));
    }

    /// Found-026: K repeated hand-outs create exactly K reservations,
    /// leave used accounting unchanged, and return distinct addresses.
    /// A later observed-use mark promotes one reservation to used.
    #[tokio::test]
    async fn found_026_repeated_handouts_create_exactly_k_reservations() {
        const K: u32 = 5;
        let wallet = wallet_with_platform_account();

        let baseline = {
            let wm = wallet.wallet_manager.read().await;
            let info = wm
                .get_wallet_info(&wallet.wallet_id)
                .expect("wallet present");
            let pool = &info
                .core_wallet
                .platform_payment_managed_account_at_index(ACCOUNT_KEY.account)
                .expect("managed account")
                .addresses;
            pool.stats()
        };

        let mut seen = BTreeSet::new();
        for _ in 0..K {
            let addr = wallet
                .next_unused_receive_address(ACCOUNT_KEY)
                .await
                .expect("hand-out");
            assert!(seen.insert(addr), "duplicate address handed out");
        }
        assert_eq!(seen.len(), K as usize);

        let mut wm = wallet.wallet_manager.write().await;
        let (_, info) = wm
            .get_wallet_mut_and_info_mut(&wallet.wallet_id)
            .expect("wallet present");
        let pool = &mut info
            .core_wallet
            .platform_payment_managed_account_at_index_mut(ACCOUNT_KEY.account)
            .expect("managed account")
            .addresses;

        let after_handouts = pool.stats();
        assert_eq!(
            after_handouts.reserved_count,
            baseline.reserved_count + K,
            "each hand-out must add exactly one reservation"
        );
        assert_eq!(
            after_handouts.used_count, baseline.used_count,
            "hand-outs must not count as observed use"
        );
        assert_eq!(pool.highest_used, baseline.highest_used);
        assert_eq!(pool.used_indices.len(), baseline.used_count as usize);

        assert!(
            pool.mark_index_used(0),
            "observed funding must promote a reservation to used"
        );
        let after_observed_use = pool.stats();
        assert_eq!(
            after_observed_use.reserved_count,
            after_handouts.reserved_count - 1
        );
        assert_eq!(after_observed_use.used_count, after_handouts.used_count + 1);
        assert_eq!(pool.highest_used, Some(0));

        assert!(
            !pool.mark_index_used(0),
            "re-marking an already-used index must remain idempotent"
        );
        assert_eq!(pool.stats().used_count, after_observed_use.used_count);
    }
}

#[cfg(test)]
mod tests {
    use super::{merge_platform_payment_candidate_addresses, PlatformAddressWallet};
    use key_wallet::PlatformP2PKHAddress;

    #[test]
    fn candidate_union_keeps_hydrated_balance_only_address_and_deduplicates_overlap() {
        let derived_only = PlatformP2PKHAddress::new([1; 20]);
        let present_in_both = PlatformP2PKHAddress::new([2; 20]);
        let hydrated_balance_only = PlatformP2PKHAddress::new([3; 20]);

        let merged = merge_platform_payment_candidate_addresses(
            [derived_only, present_in_both],
            [present_in_both, hydrated_balance_only],
        );

        assert_eq!(
            merged,
            std::collections::BTreeSet::from([
                derived_only,
                present_in_both,
                hydrated_balance_only,
            ])
        );
    }

    /// Build a `PlatformAddressWallet` on a mock SDK for getter tests that
    /// touch no I/O. Mirrors `transfer::tests::build_short_circuit_wallet`,
    /// duplicated here because that helper is private to the transfer
    /// module's `tests`.
    fn build_test_wallet() -> PlatformAddressWallet {
        use crate::broadcaster::SpvBroadcaster;
        use crate::events::PlatformEventManager;
        use crate::spv::SpvRuntime;
        use crate::wallet::asset_lock::manager::AssetLockManager;
        use crate::wallet::persister::{NoPlatformPersistence, WalletPersister};
        use std::sync::Arc;
        use tokio::sync::{Notify, RwLock};

        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let wallet_manager = Arc::new(RwLock::new(key_wallet_manager::WalletManager::new(
            sdk.network,
        )));
        let persister = WalletPersister::new([0u8; 32], Arc::new(NoPlatformPersistence));
        let event_manager = Arc::new(PlatformEventManager::new(Vec::new()));
        let spv = Arc::new(SpvRuntime::new(Arc::clone(&wallet_manager), event_manager));
        let broadcaster = Arc::new(SpvBroadcaster::new(spv));
        let asset_locks = Arc::new(AssetLockManager::new(
            Arc::clone(&sdk),
            Arc::clone(&wallet_manager),
            [0u8; 32],
            Arc::new(Notify::new()),
            broadcaster,
            persister.clone(),
        ));
        PlatformAddressWallet::new(sdk, wallet_manager, [0u8; 32], asset_locks, persister)
    }

    /// `min_input_amount()` must return the constant from the wallet's own
    /// SDK-resolved `PlatformVersion`, i.e. exactly
    /// `version.dpp.state_transitions.address_funds.min_input_amount` — the
    /// same floor the auto-selectors use to drop dust. Pins the getter to
    /// the version's value rather than a hardcoded literal, so the UI gate
    /// stays version-locked.
    #[test]
    fn min_input_amount_matches_sdk_version_constant() {
        let wallet = build_test_wallet();
        let expected = wallet
            .sdk
            .version()
            .dpp
            .state_transitions
            .address_funds
            .min_input_amount;
        assert_eq!(wallet.min_input_amount(), expected);
    }

    /// `min_output_amount()` must likewise return the constant from the
    /// wallet's own SDK-resolved `PlatformVersion`, i.e. exactly
    /// `version.dpp.state_transitions.address_funds.min_output_amount` — the
    /// per-output floor DPP enforces on address-funds transitions. Pins the
    /// getter to the version's value rather than a hardcoded literal so the
    /// transfer UI gate stays version-locked.
    #[test]
    fn min_output_amount_matches_sdk_version_constant() {
        let wallet = build_test_wallet();
        let expected = wallet
            .sdk
            .version()
            .dpp
            .state_transitions
            .address_funds
            .min_output_amount;
        assert_eq!(wallet.min_output_amount(), expected);
    }
}
