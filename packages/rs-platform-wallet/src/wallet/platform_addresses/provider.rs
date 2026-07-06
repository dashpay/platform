//! DIP-17 platform payment address provider for HD wallet scanning.
//!
//! A single [`PlatformPaymentAddressProvider`] covers **every** platform
//! payment account across **every** wallet registered on the platform
//! wallet manager, so that one call into
//! [`Sdk::sync_address_balances`] performs a single
//! trunk/branch/compact scan covering the union of all tracked
//! addresses (the "BLAST sync"). One big scan instead of N per-wallet
//! × M per-account scans lets the GroveDB proof cover many addresses
//! with one round of queries and reuses network round trips across
//! wallets.
//!
//! The pending set is a single [`bimap::BiBTreeMap`] keyed by
//! `(wallet_id, account_index, address_index)` on the left and the
//! [`PlatformP2PKHAddress`] on the right. That bijection lets
//! `on_address_found` / `on_address_absent` resolve the SDK's flat
//! `AddressIndex` callback back to a `(wallet, account, index)` triple
//! in one `remove_by_right`. The bijection is sound because different
//! accounts (even across wallets) derive from different xpubs — a given
//! address belongs to at most one `(wallet, account, index)` slot.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use bimap::BiBTreeMap;
use key_wallet::bip32::ExtendedPubKey;
use key_wallet::managed_account::address_pool::KeySource;
use key_wallet::PlatformP2PKHAddress;

use async_trait::async_trait;
use key_wallet_manager::WalletManager;

use crate::error::PlatformWalletError;
use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
use crate::PlatformAddressBalanceEntry;
use dash_sdk::platform::address_sync::{
    AddressFunds, AddressIndex, AddressProvider, AddressSyncResult,
};
use dash_sdk::query_types::AddressInfos;
use dpp::address_funds::PlatformAddress;
use tokio::sync::RwLock;

/// DIP-17 address coordinates used as both the pending-bimap key and
/// the SDK sync engine's `Tag`. Having the SDK carry these three
/// identifiers through its callback means `on_address_found` /
/// `on_address_absent` don't need to reverse-lookup by
/// [`PlatformP2PKHAddress`] to find which wallet and account an
/// address belongs to.
pub type PlatformAddressTag = (WalletId, u32, AddressIndex);

/// Gap limit used across every platform payment account until we
/// plumb per-account gap limits through again. Matches the default
/// key-wallet AddressPool setting.
const DEFAULT_GAP_LIMIT: u32 = 20;

/// Per-account bookkeeping. The wallet-level container is a plain
/// `BTreeMap<u32, PerAccountPlatformAddressState>` keyed by DIP-17
/// account index — each value holds the xpub plus the post-pass
/// artifacts for that account.
#[derive(Debug)]
pub struct PerAccountPlatformAddressState {
    /// Public-key material for this account — the xpub that
    /// gap-limit extension (inside
    /// [`PlatformPaymentAddressProvider::on_address_found`]) needs to
    /// produce a [`KeySource::Public`] on demand.
    extended_public_key: ExtendedPubKey,
    /// Every address this account has derived, as a bijection between
    /// the DIP-17 derivation index and the P2PKH address. Lets
    /// `found` / `absent` store bare [`PlatformP2PKHAddress`]
    /// values and still recover the index when building
    /// `current_balances` tuples.
    addresses: BiBTreeMap<AddressIndex, PlatformP2PKHAddress>,
    /// Addresses proven present with their balance. Persists across
    /// syncs — `on_address_found` overwrites entries when balances
    /// change, unchanged entries simply stay current, and the SDK
    /// reads directly from here via `current_balances()` to seed the
    /// next incremental pass.
    found: BTreeMap<PlatformP2PKHAddress, AddressFunds>,
    /// Addresses proven absent in the most recent sync pass. Cleared
    /// at the start of every pass — "absent" is point-in-time; an
    /// address may be present next time.
    absent: BTreeSet<PlatformP2PKHAddress>,
}

impl PerAccountPlatformAddressState {
    fn new(extended_public_key: ExtendedPubKey) -> Self {
        Self {
            extended_public_key,
            addresses: BiBTreeMap::new(),
            found: BTreeMap::new(),
            absent: BTreeSet::new(),
        }
    }

    /// Rebuild from persisted state — xpub + known derived addresses
    /// + known balances from prior syncs. `absent` starts empty
    /// because it's point-in-time per-pass information.
    pub fn from_persisted(
        extended_public_key: ExtendedPubKey,
        addresses: BiBTreeMap<AddressIndex, PlatformP2PKHAddress>,
        found: BTreeMap<PlatformP2PKHAddress, AddressFunds>,
    ) -> Self {
        Self {
            extended_public_key,
            addresses,
            found,
            absent: BTreeSet::new(),
        }
    }

    /// Seed one persisted address/funds entry into the account state.
    ///
    /// Guards the `index <-> address` bijection against an
    /// index-conflicting *removal remnant*.
    /// [`commit_reconciliation`](PlatformPaymentAddressProvider::commit_reconciliation)
    /// can zero an address whose pool-resolved `address_index` equals a
    /// *funded* address's true index; it leaves the in-memory bijection
    /// untouched but still emits the zero, so the durable store can hold
    /// two rows claiming one index. On restore those rows seed the account
    /// one at a time, and a plain [`BiBTreeMap::insert`] drops conflicting
    /// pairs — so, depending on fetch order, the zeroed row could evict the
    /// funded `(index -> address)` pairing and orphan its balance from
    /// [`current_balances`](AddressProvider::current_balances).
    ///
    /// A zero-balance/zero-nonce row can't be told apart from a legitimate
    /// freshly-derived, never-funded address by its funds alone (both are
    /// `{0, 0}`), and the latter must still restore into `found` and the
    /// bijection. So `found` is seeded for *every* row exactly as before;
    /// the guard is only on the bijection, and only bites on a collision:
    /// a zeroed row inserts via `insert_no_overwrite` (it can never evict
    /// an incumbent pairing), while a funded row keeps overwrite semantics
    /// (it wins its slot, displacing a stale zero remnant that grabbed it
    /// first — that remnant's dangling `found` entry is inert, since
    /// `current_balances` only yields addresses still paired in the
    /// bijection). For the common case of unique indices this is identical
    /// to the previous unconditional insert.
    pub fn insert_persisted_entry(
        &mut self,
        address_index: AddressIndex,
        address: PlatformP2PKHAddress,
        funds: AddressFunds,
    ) {
        self.found.insert(address, funds);
        if funds.balance == 0 && funds.nonce == 0 {
            // Never evict an incumbent pairing (funded, or another zero).
            let _ = self.addresses.insert_no_overwrite(address_index, address);
        } else {
            // Funded rows are authoritative for their index.
            self.addresses.insert(address_index, address);
        }
    }

    /// Read-only view of the persisted `(address, funds)` entries.
    ///
    /// Used by `PlatformAddressWallet::initialize_from_persisted` to
    /// push the persisted balances onto each `ManagedPlatformAccount`
    /// before the provider takes over — without this, spend paths
    /// that enumerate funded addresses (e.g.
    /// `shielded_shield_from_account`) read `available = 0` after a
    /// restart until the first BLAST sync repopulates the in-memory
    /// `address_balances` map.
    pub fn found(&self) -> &BTreeMap<PlatformP2PKHAddress, AddressFunds> {
        &self.found
    }

    /// Read-only view of the `index <-> address` bijection this account
    /// has derived.
    pub fn addresses(&self) -> &BiBTreeMap<AddressIndex, PlatformP2PKHAddress> {
        &self.addresses
    }

    /// The account xpub used to extend the gap window on demand.
    pub fn extended_public_key(&self) -> &ExtendedPubKey {
        &self.extended_public_key
    }
}

/// Per-wallet account map — keys are DIP-17 account indexes (hardened
/// level), values carry the account-level state.
pub type PerWalletPlatformAddressState = BTreeMap<u32, PerAccountPlatformAddressState>;

/// Per-account scratch state accumulated during a single sync pass.
/// Lives in [`PlatformPaymentAddressProvider::per_wallet_in_sync`]
/// while the SDK is calling our callbacks; [`sync_finished`] flushes
/// it into the committed [`PerAccountPlatformAddressState`] and
/// clears it.
///
/// Keeping found/absent writes out of the committed state until the
/// SDK signals success means a mid-sync abort leaves
/// `per_wallet.{found,absent}` intact.
#[derive(Default)]
pub(crate) struct PerAccountInSyncPlatformAddressState {
    /// Addresses the current sync pass has proven present, with
    /// their fresh funds.
    pub(crate) found: BTreeMap<PlatformP2PKHAddress, AddressFunds>,
    /// Addresses the current sync pass has proven absent.
    pub(crate) absent: BTreeSet<PlatformP2PKHAddress>,
}

/// Per-wallet in-sync scratch map, shape-parallel to
/// [`PerWalletPlatformAddressState`].
pub(crate) type PerWalletInSyncPlatformAddressState =
    BTreeMap<u32, PerAccountInSyncPlatformAddressState>;

/// Address provider covering every platform payment account across
/// every registered wallet.
///
/// Implements the SDK's [`AddressProvider`] trait by presenting a flat
/// view of pending addresses spanning all wallets.
pub(crate) struct PlatformPaymentAddressProvider {
    /// Shared wallet manager for gap-limit extension.
    wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    /// Committed per-wallet tracked state — xpub + `addresses` bimap
    /// + `found` + `absent` from the last successful sync. `found`
    /// here is what [`current_balances`](Self::current_balances)
    /// hands back to the SDK to seed the next incremental pass.
    per_wallet: BTreeMap<WalletId, PerWalletPlatformAddressState>,
    /// Scratch `found`/`absent` for the sync pass currently in
    /// flight. The SDK's `on_address_found` / `on_address_absent`
    /// callbacks mutate this map instead of [`per_wallet`]; on
    /// [`sync_finished`](AddressProvider::sync_finished) the engine
    /// flushes the scratch into the committed state and clears it.
    /// If a sync aborts mid-way, `per_wallet` stays clean.
    per_wallet_in_sync: BTreeMap<WalletId, PerWalletInSyncPlatformAddressState>,
    /// Pending addresses across every wallet/account, stored as a
    /// bijection so `on_address_found` can resolve the flat
    /// `AddressIndex` callback back to the full
    /// `(wallet, account, index)` triple in one lookup.
    pending: BiBTreeMap<PlatformAddressTag, PlatformP2PKHAddress>,
    /// Incremental watermark — one global value shared across every
    /// wallet and account, since the SDK scan is one pass across the
    /// combined pending set.
    sync_height: u64,
    sync_timestamp: u64,
    last_known_recent_block: u64,
}

impl PlatformPaymentAddressProvider {
    /// The committed per-account index/balance state for one wallet, or
    /// `None` if the provider doesn't cover it. Exposes the full persisted
    /// `index <-> address` bijection — including addresses restored from
    /// disk that are no longer in a live derived pool — so callers can map a
    /// spent address back to its derivation index.
    pub(crate) fn per_wallet_state(
        &self,
        wallet_id: &WalletId,
    ) -> Option<&PerWalletPlatformAddressState> {
        self.per_wallet.get(wallet_id)
    }

    /// Build a provider covering every platform payment account on
    /// each wallet in `wallet_ids`.
    ///
    /// Reads pre-generated addresses from each account's `AddressPool`;
    /// no key derivation happens here. `wallet_ids` not found in the
    /// wallet manager are silently skipped.
    pub(crate) async fn from_wallets(
        wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        wallet_ids: impl IntoIterator<Item = WalletId>,
    ) -> Result<Self, PlatformWalletError> {
        let mut per_wallet: BTreeMap<WalletId, PerWalletPlatformAddressState> = BTreeMap::new();
        let mut pending: BiBTreeMap<PlatformAddressTag, PlatformP2PKHAddress> = BiBTreeMap::new();

        {
            let wm = wallet_manager.read().await;
            for wallet_id in wallet_ids {
                let Some((wallet, info)) = wm.get_wallet_and_info(&wallet_id) else {
                    tracing::warn!(
                        "from_wallets: wallet {:?} not found, skipping",
                        hex::encode(wallet_id)
                    );
                    continue;
                };

                let mut state = PerWalletPlatformAddressState::new();

                for (&account_key, account_xpub) in wallet
                    .accounts
                    .platform_payment_accounts
                    .iter()
                    .map(|(k, v)| (k, v.account_xpub))
                {
                    let account_index = account_key.account;
                    if state.contains_key(&account_index) {
                        continue;
                    }

                    let Some(managed) = info
                        .core_wallet
                        .platform_payment_managed_account_at_index(account_index)
                    else {
                        // Missing managed state shouldn't happen if the
                        // key is in the HD account map, but skip
                        // defensively.
                        continue;
                    };

                    let mut account_state = PerAccountPlatformAddressState::new(account_xpub);
                    for (&index, addr_info) in &managed.addresses.addresses {
                        let Ok(p2pkh) = PlatformP2PKHAddress::from_address(&addr_info.address)
                        else {
                            continue;
                        };
                        pending.insert((wallet_id, account_index, index), p2pkh);
                        account_state.addresses.insert(index, p2pkh);
                    }

                    state.insert(account_index, account_state);
                }

                per_wallet.insert(wallet_id, state);
            }
        }

        Ok(Self {
            wallet_manager,
            per_wallet,
            per_wallet_in_sync: BTreeMap::new(),
            pending,
            sync_height: 0,
            sync_timestamp: 0,
            last_known_recent_block: 0,
        })
    }

    /// Rebuild a provider from persisted per-wallet state and an
    /// incremental-sync watermark. Used on startup when a persister
    /// has state to restore — the caller supplies the xpubs, known
    /// derived addresses, and the `found` balance map per account.
    /// This constructor preserves the persisted address map and
    /// merges in any newer addresses currently present in the live
    /// managed-account pools.
    ///
    /// Returns an error if the persisted state references a wallet
    /// or platform payment account that isn't in the live wallet
    /// manager. Those shouldn't drift in practice; if they do, it
    /// means the wallet store and the persisted BLAST state are out
    /// of sync and the caller needs to reconcile rather than silently
    /// continue with stale data.
    pub async fn from_persisted(
        wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        per_wallet: BTreeMap<WalletId, PerWalletPlatformAddressState>,
        sync_height: u64,
        sync_timestamp: u64,
        last_known_recent_block: u64,
    ) -> Result<Self, PlatformWalletError> {
        let mut new_per_wallet: BTreeMap<WalletId, PerWalletPlatformAddressState> = BTreeMap::new();
        let mut pending: BiBTreeMap<PlatformAddressTag, PlatformP2PKHAddress> = BiBTreeMap::new();

        {
            let wm = wallet_manager.read().await;
            for (wallet_id, wallet_state) in per_wallet {
                let info = wm.get_wallet_info(&wallet_id).ok_or_else(|| {
                    PlatformWalletError::WalletNotFound(format!(
                        "from_persisted: wallet {} not found in wallet manager",
                        hex::encode(wallet_id)
                    ))
                })?;

                let mut new_wallet_state = PerWalletPlatformAddressState::new();
                for (account_index, mut account_state) in wallet_state {
                    let managed = info
                        .core_wallet
                        .platform_payment_managed_account_at_index(account_index)
                        .ok_or_else(|| {
                            PlatformWalletError::AddressSync(format!(
                                "from_persisted: wallet {} has no platform payment account {}",
                                hex::encode(wallet_id),
                                account_index
                            ))
                        })?;

                    // Preserve the persisted address map, then merge
                    // any newer live-pool addresses on top so startup
                    // doesn't lose addresses that fell out of the
                    // current in-memory gap window.
                    for (&index, &p2pkh) in account_state.addresses.iter() {
                        pending.insert((wallet_id, account_index, index), p2pkh);
                    }
                    for (&index, addr_info) in &managed.addresses.addresses {
                        let Ok(p2pkh) = PlatformP2PKHAddress::from_address(&addr_info.address)
                        else {
                            continue;
                        };
                        pending.insert((wallet_id, account_index, index), p2pkh);
                        account_state.addresses.insert(index, p2pkh);
                    }

                    new_wallet_state.insert(account_index, account_state);
                }
                new_per_wallet.insert(wallet_id, new_wallet_state);
            }
        }

        Ok(Self {
            wallet_manager,
            per_wallet: new_per_wallet,
            per_wallet_in_sync: BTreeMap::new(),
            pending,
            sync_height,
            sync_timestamp,
            last_known_recent_block,
        })
    }

    /// Public-key source for a given `(wallet, account)`, built on
    /// demand from the stored extended public key. Returns `None` if
    /// the `(wallet, account)` isn't tracked.
    pub(crate) fn key_source(&self, wallet_id: &WalletId, account_index: u32) -> Option<KeySource> {
        self.per_wallet
            .get(wallet_id)
            .and_then(|s| s.get(&account_index))
            .map(|a| KeySource::Public(a.extended_public_key))
    }

    /// The last sync timestamp, or `None` if never synced.
    pub(crate) fn last_sync_timestamp(&self) -> Option<u64> {
        if self.sync_timestamp == 0 {
            None
        } else {
            Some(self.sync_timestamp)
        }
    }

    /// Re-populate `pending` for every tracked wallet and account
    /// from their persisted address map plus any additional live-pool
    /// addresses, then clear the per-pass `absent` set. `found` is
    /// intentionally preserved across syncs — it doubles as the
    /// `current_balances()` seed for the next incremental round.
    ///
    /// Call before each sync round.
    pub(crate) async fn prepare_for_sync(&mut self) -> Result<(), PlatformWalletError> {
        // Drop any scratch left over from an aborted pass. The SDK only
        // calls `sync_finished` on success, so a sync that errored after
        // staging found/absent entries leaves them here — and a stale
        // staged absent would make the next successful `sync_finished`
        // remove a committed balance the new pass never re-proved
        // absent.
        self.per_wallet_in_sync.clear();

        let wallet_ids: Vec<WalletId> = self.per_wallet.keys().copied().collect();

        // Refresh provider-level pending and merge in any new
        // addresses the managed account has derived since the last
        // pass. Persisted addresses remain tracked even if the live
        // pool no longer exposes them.
        let fresh_pending: BiBTreeMap<PlatformAddressTag, PlatformP2PKHAddress> = {
            let wm = self.wallet_manager.read().await;
            let mut out: BiBTreeMap<PlatformAddressTag, PlatformP2PKHAddress> = BiBTreeMap::new();
            for wallet_id in wallet_ids {
                let Some(info) = wm.get_wallet_info(&wallet_id) else {
                    continue;
                };
                let Some(state) = self.per_wallet.get_mut(&wallet_id) else {
                    continue;
                };
                let account_indexes: Vec<u32> = state.keys().copied().collect();
                for account_index in account_indexes {
                    let Some(managed) = info
                        .core_wallet
                        .platform_payment_managed_account_at_index(account_index)
                    else {
                        continue;
                    };
                    let Some(account_state) = state.get_mut(&account_index) else {
                        continue;
                    };
                    for (&index, addr_info) in &managed.addresses.addresses {
                        let Ok(p2pkh) = PlatformP2PKHAddress::from_address(&addr_info.address)
                        else {
                            continue;
                        };
                        account_state.addresses.insert(index, p2pkh);
                    }
                    for (&index, &p2pkh) in account_state.addresses.iter() {
                        out.insert((wallet_id, account_index, index), p2pkh);
                    }
                }
            }
            out
        };

        self.pending = fresh_pending;
        for state in self.per_wallet.values_mut() {
            for account_state in state.values_mut() {
                account_state.absent.clear();
            }
        }
        Ok(())
    }

    /// Update incremental sync state from a completed sync result.
    pub(crate) fn update_sync_state(
        &mut self,
        result: &AddressSyncResult<PlatformAddressTag, PlatformP2PKHAddress>,
    ) {
        self.sync_height = result.new_sync_height;
        self.sync_timestamp = result.new_sync_timestamp;
        self.last_known_recent_block = result.last_known_recent_block;
    }

    /// Current `last_known_recent_block` watermark.
    ///
    /// Crate-visible mirror of the field used by the `AddressProvider`
    /// trait implementation, so wallet-level helpers (notably
    /// [`super::wallet::PlatformAddressWallet::sync_watermark`]) can
    /// read the value without going through the trait. Monotonic
    /// non-decreasing across `sync_finished` calls.
    pub(crate) fn last_known_recent_block(&self) -> u64 {
        self.last_known_recent_block
    }

    /// Restore incremental-sync watermark from persisted state.
    pub(crate) fn set_stored_sync_state(
        &mut self,
        height: u64,
        timestamp: u64,
        last_known_recent_block: u64,
    ) {
        self.sync_height = height;
        self.sync_timestamp = timestamp;
        self.last_known_recent_block = last_known_recent_block;
    }

    /// Reset the incremental-sync watermark and drop every cached
    /// balance so the next `sync_balances` performs a full
    /// trunk/branch/compact rescan from genesis instead of an
    /// incremental catch-up.
    ///
    /// Backs the host's "Clear" flow. Zeroing the three watermark
    /// scalars alone is not enough: `found` doubles as the
    /// `current_balances()` seed for the next pass (and the `before`
    /// snapshot for the persistence diff), so a non-empty `found`
    /// would re-seed the very balances Clear is meant to wipe.
    /// `sync_timestamp == 0` is what flips `last_sync_timestamp()`
    /// back to `None` and the SDK back into full-scan mode.
    ///
    /// The `addresses` bijection is intentionally preserved —
    /// `prepare_for_sync` rebuilds `pending` from it each pass, so
    /// keeping it avoids needless re-derivation while still forcing a
    /// full rescan.
    pub(crate) fn reset_sync_state(&mut self) {
        self.sync_height = 0;
        self.sync_timestamp = 0;
        self.last_known_recent_block = 0;
        self.per_wallet_in_sync.clear();
        for state in self.per_wallet.values_mut() {
            for account_state in state.values_mut() {
                account_state.found.clear();
                account_state.absent.clear();
            }
        }
    }

    /// Diagnostic snapshot counts used by the read-only memory
    /// explorer surface on
    /// [`crate::manager::PlatformWalletManager::platform_address_provider_state_blocking`].
    /// Returns `(accounts_watched, found_count, known_balances_count)`
    /// for `wallet_id`. Reading both `found.len()` and `addresses.len()`
    /// from the same per-account state captures the two concepts the
    /// explorer wants to surface separately.
    pub fn diagnostic_counts(&self, wallet_id: &WalletId) -> (usize, usize, usize) {
        let Some(state) = self.per_wallet.get(wallet_id) else {
            return (0, 0, 0);
        };
        let accounts_watched = state.len();
        let mut found_count = 0;
        let mut known_balances_count = 0;
        for account_state in state.values() {
            // `found` holds proven-present addresses with balances —
            // this is exactly the "currently has a balance" set the
            // SDK seeds the next pass with.
            found_count += account_state.found.len();
            // `addresses` is the bijection of every derivation index
            // we've ever tracked for this account, so its size is the
            // "known balances slot count" the explorer reports.
            known_balances_count += account_state.addresses.len();
        }
        (accounts_watched, found_count, known_balances_count)
    }

    /// Diagnostic getter — the unified-pass watermark height as a
    /// `u32` (the SDK exposes it as `u64` internally; the diagnostic
    /// surface is `u32` to match the rest of the explorer's height
    /// fields). Saturates at `u32::MAX` rather than silently wrapping
    /// — Dash core heights never reach that range in practice, so
    /// any value that would truncate is corruption / a sentinel that
    /// should surface visibly in the diagnostic panel.
    pub fn diagnostic_sync_height_u32(&self) -> u32 {
        u32::try_from(self.sync_height).unwrap_or(u32::MAX)
    }
}

#[async_trait]
impl AddressProvider for PlatformPaymentAddressProvider {
    /// The engine carries full `(wallet, account, derivation index)`
    /// coordinates through its callbacks, so we don't need to reverse-
    /// lookup from address back to wallet/account ourselves.
    type Tag = PlatformAddressTag;
    /// Platform payment accounts only derive P2PKH, so the provider
    /// uses the narrow address type directly — no enum wrap/unwrap at
    /// trait boundaries.
    type Address = PlatformP2PKHAddress;

    /// Fixed at [`DEFAULT_GAP_LIMIT`] (20) until we plumb per-account
    /// gap limits through the unified provider again.
    fn gap_limit(&self) -> AddressIndex {
        DEFAULT_GAP_LIMIT
    }

    fn pending_addresses(
        &self,
    ) -> impl Iterator<Item = (PlatformAddressTag, PlatformP2PKHAddress)> + '_ {
        self.pending.iter().map(|(&tag, &p2pkh)| (tag, p2pkh))
    }

    fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    async fn on_address_found(
        &mut self,
        tag: PlatformAddressTag,
        address: &PlatformP2PKHAddress,
        funds: AddressFunds,
    ) {
        let p2pkh = *address;
        let (wallet_id, account_index, address_index) = tag;

        // Consume the pending entry. Missing is fine — the engine can
        // call this on incremental-catch-up hits that were never
        // pending in the first place.
        self.pending.remove_by_left(&tag);

        // Validate against committed state: the SDK's tag can only
        // name an address we previously registered as pending, so if
        // the account bimap is missing it our state has drifted.
        let Some(committed) = self
            .per_wallet
            .get(&wallet_id)
            .and_then(|s| s.get(&account_index))
        else {
            tracing::warn!(
                "on_address_found: no tracked state for wallet {} account {}",
                hex::encode(wallet_id),
                account_index
            );
            return;
        };
        if !committed.addresses.contains_left(&address_index) {
            tracing::error!(
                "on_address_found: (wallet={}, account={}, index={}) missing from account bimap — state drift",
                hex::encode(wallet_id),
                account_index,
                address_index
            );
            return;
        }
        let key_source = KeySource::Public(committed.extended_public_key);

        // Stage the balance update in the in-sync scratch map —
        // `sync_finished` flushes it to the committed `per_wallet`.
        self.per_wallet_in_sync
            .entry(wallet_id)
            .or_default()
            .entry(account_index)
            .or_default()
            .found
            .insert(p2pkh, funds);

        // Update key-wallet's managed account: mark the address used,
        // set its balance, and let the AddressPool generate any new
        // addresses needed to maintain the gap limit. Collect the
        // new addresses under the write lock, then release it before
        // mutating our own maps.
        let new_addresses: Vec<(AddressIndex, PlatformP2PKHAddress)> = {
            let mut wm = self.wallet_manager.write().await;
            let Some(info) = wm.get_wallet_info_mut(&wallet_id) else {
                tracing::warn!(
                    "on_address_found: wallet {} not in wallet manager",
                    hex::encode(wallet_id)
                );
                return;
            };
            let Some(account) = info
                .core_wallet
                .platform_payment_managed_account_at_index_mut(account_index)
            else {
                tracing::warn!(
                    "on_address_found: no platform payment account {} in wallet {}",
                    account_index,
                    hex::encode(wallet_id)
                );
                return;
            };

            account.set_address_credit_balance(p2pkh, funds.balance, Some(&key_source));

            account
                .addresses
                .addresses
                .iter()
                .filter_map(|(&index, addr_info)| {
                    let key = (wallet_id, account_index, index);
                    if self.pending.contains_left(&key) {
                        return None;
                    }
                    let new_p2pkh = PlatformP2PKHAddress::from_address(&addr_info.address).ok()?;
                    Some((index, new_p2pkh))
                })
                .collect()
        };

        for (index, new_p2pkh) in new_addresses {
            self.pending
                .insert((wallet_id, account_index, index), new_p2pkh);
            if let Some(account_state) = self
                .per_wallet
                .get_mut(&wallet_id)
                .and_then(|s| s.get_mut(&account_index))
            {
                account_state.addresses.insert(index, new_p2pkh);
            }
        }
    }

    async fn on_address_absent(&mut self, tag: PlatformAddressTag, address: &PlatformP2PKHAddress) {
        let p2pkh = *address;
        let (wallet_id, account_index, address_index) = tag;

        self.pending.remove_by_left(&tag);

        let Some(committed) = self
            .per_wallet
            .get(&wallet_id)
            .and_then(|s| s.get(&account_index))
        else {
            tracing::warn!(
                "on_address_absent: no tracked state for wallet {} account {}",
                hex::encode(wallet_id),
                account_index
            );
            return;
        };
        if !committed.addresses.contains_left(&address_index) {
            tracing::error!(
                "on_address_absent: (wallet={}, account={}, index={}) missing from account bimap — state drift",
                hex::encode(wallet_id),
                account_index,
                address_index
            );
            return;
        }
        self.per_wallet_in_sync
            .entry(wallet_id)
            .or_default()
            .entry(account_index)
            .or_default()
            .absent
            .insert(p2pkh);

        // Mirror `on_address_found`'s managed-account write: zero the
        // in-memory credit balance for this address so spend paths that
        // enumerate funded addresses (notably identity-creation funding)
        // stop offering a stale balance proven absent from state. Pass
        // `None` for the key source — `set_address_credit_balance` only
        // triggers gap-limit extension when an address transitions from
        // unfunded to funded, and zeroing never does, so there is no
        // gap-limit work to drive here.
        let mut wm = self.wallet_manager.write().await;
        let Some(info) = wm.get_wallet_info_mut(&wallet_id) else {
            tracing::warn!(
                "on_address_absent: wallet {} not in wallet manager",
                hex::encode(wallet_id)
            );
            return;
        };
        let Some(account) = info
            .core_wallet
            .platform_payment_managed_account_at_index_mut(account_index)
        else {
            tracing::warn!(
                "on_address_absent: no platform payment account {} in wallet {}",
                account_index,
                hex::encode(wallet_id)
            );
            return;
        };
        account.set_address_credit_balance(p2pkh, 0, None);
    }

    async fn sync_finished(&mut self) {
        // Flush scratch state accumulated during the pass into the
        // committed per-wallet state. `found` is merged entry-by-entry
        // (new/changed balances overwrite prior values), then every
        // address proven absent this pass is REMOVED from the committed
        // `found` map; `absent` itself is replaced wholesale since it's
        // point-in-time per-pass.
        //
        // The absent-removal is what stops a stale balance from a prior
        // chain (e.g. after a devnet reset) from being re-seeded into the
        // next incremental pass and the diagnostic surface: `found` is
        // the seed `current_balances()` hands back to the SDK, so a
        // proven-absent address that lingers there keeps reporting the
        // old balance forever.
        //
        // The two scratch sets are NOT guaranteed disjoint: the full
        // scan can prove an address absent at the trunk/branch
        // checkpoint, then incremental catch-up re-finds it (a credit op
        // between checkpoint and tip stages a fresh `found` entry
        // without retracting the earlier absent proof). The catch-up
        // find reflects the chain tip, so it wins — such addresses are
        // dropped from the absent set before it drives any removal.
        let drained = std::mem::take(&mut self.per_wallet_in_sync);
        for (wallet_id, wallet_scratch) in drained {
            let Some(wallet_state) = self.per_wallet.get_mut(&wallet_id) else {
                continue;
            };
            for (account_index, mut account_scratch) in wallet_scratch {
                let Some(account_state) = wallet_state.get_mut(&account_index) else {
                    continue;
                };
                let PerAccountInSyncPlatformAddressState { found, absent } = &mut account_scratch;
                absent.retain(|addr| !found.contains_key(addr));
                // Height-pin freshness on the merge: a pass that ran
                // against a lagging node can stage a stale-but-valid
                // absolute for a row a fresher reconcile already
                // committed (its pin is older). Keeping the fresher
                // committed entry mirrors `commit_reconciliation`'s
                // rule AND `compute_address_balance_diff`'s persist
                // guard, so the in-memory seed and the durable rows
                // never diverge over which of the two is truth.
                for (addr, incoming) in account_scratch.found {
                    match account_state.found.get(&addr) {
                        Some(existing) if existing.as_of_height > incoming.as_of_height => {}
                        _ => {
                            account_state.found.insert(addr, incoming);
                        }
                    }
                }
                // Absence carries no per-entry height, so a stale pass's
                // removal is NOT pin-guarded here; the persist diff skips
                // the durable zero for fresher-pinned rows, and the next
                // pass reconstructs the in-memory entry from a full
                // replay (base 0, pin 0 → every delta applies).
                for absent_addr in &account_scratch.absent {
                    account_state.found.remove(absent_addr);
                }
                account_state.absent = account_scratch.absent;
            }
        }
    }

    fn current_balances(
        &self,
    ) -> impl Iterator<Item = (PlatformAddressTag, PlatformP2PKHAddress, AddressFunds)> + '_ {
        self.per_wallet.iter().flat_map(|(wallet_id, state)| {
            state
                .iter()
                .flat_map(move |(&account_index, account_state)| {
                    account_state
                        .found
                        .iter()
                        .filter_map(move |(p2pkh, &funds)| {
                            let &address_index = account_state.addresses.get_by_right(p2pkh)?;
                            Some(((*wallet_id, account_index, address_index), *p2pkh, funds))
                        })
                })
        })
    }

    fn last_sync_height(&self) -> u64 {
        self.sync_height
    }

    fn last_known_recent_block_height(&self) -> u64 {
        self.last_known_recent_block
    }
}

/// Translate a state transition's proof-attested `address_infos` into
/// persistence-changeset entries, resolving each address's
/// `(account_index, address_index)` through `resolve_index`.
///
/// Non-P2PKH addresses and addresses the resolver doesn't recognise
/// (external recipients, or addresses the wallet doesn't own) are skipped.
/// Missing per-address info (`None`) maps to zero balance / zero nonce —
/// the on-chain post-transition state for an address removed from state
/// (e.g. a fully consumed input). Pure and lock-free so every caller's
/// translation is unit-testable.
///
/// `as_of_height` is the proof's block height: every produced entry is an
/// absolute attested at that height, so its funds carry it as the height
/// pin (see `AddressFunds::as_of_height`) — including removals, which are
/// equally height-attested statements.
///
/// Callers supply the resolver so every reconciliation path can resolve
/// through the provider's persisted `index <-> address` bijection —
/// covering addresses restored from disk that are no longer in a live
/// derived pool — with the live pool as fallback for addresses derived
/// since the last sync (see
/// [`PlatformPaymentAddressProvider::commit_reconciliation`]).
pub(crate) fn build_address_balance_entries(
    wallet_id: WalletId,
    resolve_index: impl Fn(&PlatformP2PKHAddress) -> Option<(u32, AddressIndex)>,
    address_infos: &AddressInfos,
    as_of_height: u64,
) -> Vec<PlatformAddressBalanceEntry> {
    let mut entries = Vec::new();
    for (addr, maybe_info) in address_infos.iter() {
        let PlatformAddress::P2pkh(hash) = addr else {
            continue;
        };
        let p2pkh = PlatformP2PKHAddress::new(*hash);
        let Some((account_index, address_index)) = resolve_index(&p2pkh) else {
            continue;
        };
        let funds = match maybe_info {
            Some(ai) => AddressFunds {
                balance: ai.balance,
                nonce: ai.nonce,
                as_of_height,
            },
            None => AddressFunds {
                balance: 0,
                nonce: 0,
                as_of_height,
            },
        };
        entries.push(PlatformAddressBalanceEntry {
            wallet_id,
            account_index,
            address_index,
            address: p2pkh,
            funds,
        });
    }
    entries
}

/// What [`PlatformPaymentAddressProvider::commit_reconciliation`] did with
/// a proof-attested `address_infos` map: the entries that survived the
/// freshness guard (already committed to the provider's `found` seed),
/// plus counters so the caller can log why entries were dropped.
#[derive(Default)]
pub(crate) struct ReconciliationOutcome {
    /// Entries to apply to the managed accounts and persist. Already
    /// committed to the provider's `found` map / bijection.
    pub(crate) entries: Vec<PlatformAddressBalanceEntry>,
    /// How many proof addresses resolved to a wallet-owned slot at all
    /// (before the freshness guard). Zero with a non-empty proof means
    /// either every address belongs to a third party or resolution failed.
    pub(crate) resolved: usize,
    /// Resolved entries dropped because the committed `found` seed already
    /// carries a higher nonce — a background sync (or a later transition)
    /// committed fresher state after this proof was produced.
    pub(crate) stale_skipped: usize,
    /// Resolved entries dropped as no-ops (funds identical to the
    /// committed seed) to avoid persister churn.
    pub(crate) unchanged_skipped: usize,
}

impl PlatformPaymentAddressProvider {
    /// Resolve a state transition's proof-attested `address_infos` for
    /// `wallet_id`, apply the freshness guard, and commit the survivors to
    /// the provider's `found` map (the sync-diff baseline and the seed
    /// [`current_balances`](AddressProvider::current_balances) hands the
    /// SDK) so reconciliation and the background sync cannot diverge.
    ///
    /// Resolution goes through the persisted `index <-> address` bijection
    /// first — covering addresses restored from disk that are no longer in
    /// a live derived pool — then falls back to `pool_indexes` (the live
    /// pools, covering addresses derived since the last sync, e.g. a fresh
    /// change address). The fallback is restricted to accounts the
    /// provider tracks: every emitted entry is committed to the `found`
    /// seed, so an account without per-account provider state resolves
    /// to nothing (it reconciles once `initialize` / `add_provider`
    /// re-snapshots the account set and the next sync runs).
    /// Pool-resolved addresses are merged into the bijection so
    /// `current_balances` can yield their committed funds.
    ///
    /// Freshness guard, per resolved entry — height-pin authority (see
    /// `AddressFunds::as_of_height`):
    /// * an entry whose pin is *below* the committed seed's pin is stale —
    ///   a sync pass or later transition already committed state attested
    ///   at a later block — and is dropped. This applies to removals too:
    ///   an older removal proof must not clobber a newer re-credit.
    /// * on equal pins (same block), the nonce breaks the tie — it only
    ///   advances on outgoing ops, so it can order same-block states but
    ///   not receive-only states across blocks (the pin does that);
    /// * entries identical to the committed seed are dropped as no-ops.
    ///
    /// Zero funds (balance 0, nonce 0 — the address was removed from
    /// Platform state, e.g. a fully consumed input) that survive the guard
    /// drop the address from `found`, mirroring the sync's absent handling.
    ///
    /// `as_of_height` is the proof's block height and becomes the pin on
    /// every committed entry.
    ///
    /// Callers must hold the provider write lock (i.e. call through
    /// `&mut self`) across this commit AND the managed-account balance
    /// write that follows, so a background sync — which holds the same
    /// lock across its scan — can never interleave between the two.
    pub(crate) fn commit_reconciliation(
        &mut self,
        wallet_id: &WalletId,
        address_infos: &AddressInfos,
        pool_indexes: &BTreeMap<PlatformP2PKHAddress, (u32, AddressIndex)>,
        as_of_height: u64,
    ) -> ReconciliationOutcome {
        let mut outcome = ReconciliationOutcome::default();
        let Some(wallet_state) = self.per_wallet.get_mut(wallet_id) else {
            return outcome;
        };

        let resolved_entries = build_address_balance_entries(
            *wallet_id,
            |p2pkh| {
                for (&account_index, account_state) in wallet_state.iter() {
                    if let Some(&address_index) = account_state.addresses.get_by_right(p2pkh) {
                        return Some((account_index, address_index));
                    }
                }
                // Pool fallback only for accounts the provider tracks —
                // an account added to the live wallet after the provider
                // snapshot has no per-account state to commit into, so
                // resolving it here would break the contract that every
                // emitted entry is committed to the `found` seed. Such
                // addresses stay unresolved until `initialize` /
                // `add_provider` re-snapshots the account set.
                pool_indexes
                    .get(p2pkh)
                    .copied()
                    .filter(|(account_index, _)| wallet_state.contains_key(account_index))
            },
            address_infos,
            as_of_height,
        );
        outcome.resolved = resolved_entries.len();

        for entry in resolved_entries {
            // The resolver only yields accounts present in `wallet_state`,
            // so a miss here is unreachable; skip defensively rather than
            // emit an entry the seed never committed.
            let Some(state) = wallet_state.get_mut(&entry.account_index) else {
                tracing::warn!(
                    account_index = entry.account_index,
                    address = %entry.address,
                    "commit_reconciliation: resolved account has no tracked \
                     provider state; dropping entry"
                );
                continue;
            };
            let existing = state.found.get(&entry.address).copied();
            // Zero funds = the address no longer exists in Platform state
            // (e.g. a fully consumed input); survivors of the freshness
            // guard drop the address from `found` below.
            let is_removal = entry.funds.balance == 0 && entry.funds.nonce == 0;
            if let Some(existing) = existing {
                // Height-pin authority: a committed absolute pinned at a
                // later block supersedes this proof — removals included
                // (an older removal must not clobber a newer re-credit).
                // Equal pins (same block) fall back to the nonce, which
                // orders same-block outgoing ops. Legacy pin-0 rows lose
                // to any pinned proof, which is the self-healing path for
                // state persisted before the pin existed.
                let stale = existing.as_of_height > entry.funds.as_of_height
                    || (existing.as_of_height == entry.funds.as_of_height
                        && existing.nonce > entry.funds.nonce);
                if stale {
                    outcome.stale_skipped += 1;
                    continue;
                }
                if existing == entry.funds {
                    outcome.unchanged_skipped += 1;
                    continue;
                }
            }
            // Derivation-index conflict: `entry.address` isn't yet in the
            // bijection, but its `address_index` already maps to a
            // DIFFERENT address. Detected BEFORE the `found` mutation
            // because a conflicting credit must not be half-applied.
            let index_conflict = state.addresses.get_by_right(&entry.address).is_none()
                && state.addresses.contains_left(&entry.address_index);

            if index_conflict && !is_removal {
                // A credit under a conflicting index: dropping it outright
                // is the only safe response. Inserting the pairing would
                // evict the existing one (`BiBTreeMap::insert` drops
                // conflicting pairs, orphaning the other address's `found`
                // entry); NOT inserting it would commit a `found` balance
                // downstream can't pair with a derivation index, so
                // `current_balances` couldn't round-trip the committed
                // seed. The address stays unresolved until `initialize` /
                // `add_provider` re-snapshots the account set.
                tracing::error!(
                    account_index = entry.account_index,
                    address_index = entry.address_index,
                    address = %entry.address,
                    "commit_reconciliation: derivation index already maps to a \
                     different address — dropping the credit reconciliation entry \
                     to avoid corrupting the bijection"
                );
                continue;
            }

            if is_removal {
                state.found.remove(&entry.address);
            } else {
                state.found.insert(entry.address, entry.funds);
            }

            if index_conflict {
                // A removal under a conflicting index: the credit case
                // already `continue`d above, so this is a zero-out. It
                // MUST still zero `found` (done) and be emitted (below) so
                // the durable persister writes the zero — otherwise a
                // stale persisted balance for this address resurrects
                // after restart. Only the bijection merge is skipped, so
                // the pre-existing `(index -> other address)` pairing
                // survives.
                tracing::warn!(
                    account_index = entry.account_index,
                    address_index = entry.address_index,
                    address = %entry.address,
                    "commit_reconciliation: derivation index already maps to a \
                     different address — applying the removal without touching \
                     the bijection so a stale persisted balance can't resurrect"
                );
            } else if state.addresses.get_by_right(&entry.address).is_none() {
                // Merge pool-resolved addresses into the bijection so
                // `current_balances` can pair the fresh funds with a
                // derivation index. The conflict guard above ruled out an
                // eviction, so this insert is always a fresh pairing.
                state.addresses.insert(entry.address_index, entry.address);
            }
            outcome.entries.push(entry);
        }
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashcore::secp256k1::Secp256k1;
    use key_wallet::bip32::ExtendedPrivKey;
    use key_wallet::Network;
    use key_wallet_manager::WalletManager;

    const WALLET: WalletId = [3u8; 32];
    const ACCOUNT: u32 = 0;

    fn test_xpub() -> ExtendedPubKey {
        let secp = Secp256k1::new();
        let seed = [42u8; 32];
        let xprv = ExtendedPrivKey::new_master(Network::Testnet, &seed).expect("master xprv");
        ExtendedPubKey::from_priv(&secp, &xprv)
    }

    fn p2pkh(byte: u8) -> PlatformP2PKHAddress {
        PlatformP2PKHAddress::new([byte; 20])
    }

    fn funds(balance: u64, nonce: u32) -> AddressFunds {
        AddressFunds {
            balance,
            nonce,
            as_of_height: 0,
        }
    }

    fn funds_at(balance: u64, nonce: u32, as_of_height: u64) -> AddressFunds {
        AddressFunds {
            balance,
            nonce,
            as_of_height,
        }
    }

    /// Regression for the top-up reconciliation: a spent platform address
    /// that exists only in the persisted index bijection (restored from
    /// disk — not in any live derived pool) must still be resolved to its
    /// derivation index and recorded with the proof's post-spend balance.
    /// The original fix scanned only the live pool, so it left restored
    /// rows stale, preserving the phantom Platform Balance the PR targets.
    #[test]
    fn build_entries_resolves_restored_address_outside_live_pool() {
        use dash_sdk::query_types::AddressInfo;

        // Present in the persisted bijection at index 3, but NOT in a live
        // derived address pool.
        let restored = p2pkh(0x11);
        let mut bimap: BiBTreeMap<AddressIndex, PlatformP2PKHAddress> = BiBTreeMap::new();
        bimap.insert(3, restored);

        // The top-up spent it; the proof attests post-spend balance 5,
        // nonce bumped to 4.
        let restored_addr = PlatformAddress::P2pkh([0x11; 20]);
        let mut address_infos = AddressInfos::new();
        address_infos.insert(
            restored_addr,
            Some(AddressInfo {
                address: restored_addr,
                nonce: 4,
                balance: 5,
            }),
        );

        let entries = build_address_balance_entries(
            WALLET,
            |p2pkh| bimap.get_by_right(p2pkh).map(|&idx| (ACCOUNT, idx)),
            &address_infos,
            42,
        );

        assert_eq!(
            entries.len(),
            1,
            "a restored spent address must still be reconciled"
        );
        let e = &entries[0];
        assert_eq!(e.address, restored);
        assert_eq!(e.account_index, ACCOUNT);
        assert_eq!(
            e.address_index, 3,
            "index resolved from the persisted bijection, not the live pool"
        );
        assert_eq!(
            e.funds.balance, 5,
            "records the proof's post-spend balance, not a stale value"
        );
        assert_eq!(e.funds.nonce, 4, "records the bumped nonce");
    }

    /// `transfer_address_funds` returns address info for the full
    /// `inputs ∪ outputs` set, including external recipients the wallet
    /// does not own. The builder must keep entries only for addresses the
    /// resolver recognises — persisting a recipient under a fabricated
    /// derivation index would poison the account's address map on restore.
    /// Missing per-address info maps to zero funds (the post-transition
    /// state for a fully consumed input elided from the proved set).
    #[test]
    fn build_entries_drops_unresolved_and_zeroes_missing_info() {
        use dash_sdk::query_types::AddressInfo;

        let owned = p2pkh(0x01);
        let mut bimap: BiBTreeMap<AddressIndex, PlatformP2PKHAddress> = BiBTreeMap::new();
        bimap.insert(7, owned);

        let owned_addr = PlatformAddress::P2pkh([0x01; 20]);
        let external_addr = PlatformAddress::P2pkh([0xEE; 20]);
        let mut address_infos = AddressInfos::new();
        // Fully consumed input: drive elides the info.
        address_infos.insert(owned_addr, None);
        // External recipient: resolver won't know it.
        address_infos.insert(
            external_addr,
            Some(AddressInfo {
                address: external_addr,
                nonce: 0,
                balance: 5_000_000,
            }),
        );

        let entries = build_address_balance_entries(
            WALLET,
            |p2pkh| bimap.get_by_right(p2pkh).map(|&idx| (ACCOUNT, idx)),
            &address_infos,
            42,
        );

        assert_eq!(entries.len(), 1, "external recipient must be filtered out");
        let e = &entries[0];
        assert_eq!(e.address, owned);
        assert_eq!(e.address_index, 7);
        assert_eq!(e.funds.balance, 0, "missing info means removed from state");
        assert_eq!(e.funds.nonce, 0);
    }

    /// Build a provider whose committed `per_wallet` tracks a single
    /// account on `wallet_id` with one funded address (index 0), backed
    /// by the supplied wallet manager.
    fn provider_tracking_address(
        wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        wallet_id: WalletId,
        addr: PlatformP2PKHAddress,
        f: AddressFunds,
    ) -> PlatformPaymentAddressProvider {
        let mut account_state = PerAccountPlatformAddressState::from_persisted(
            test_xpub(),
            BiBTreeMap::new(),
            BTreeMap::new(),
        );
        account_state.insert_persisted_entry(0, addr, f);

        let mut wallet_state = PerWalletPlatformAddressState::new();
        wallet_state.insert(ACCOUNT, account_state);

        let mut per_wallet = BTreeMap::new();
        per_wallet.insert(wallet_id, wallet_state);

        let mut pending = BiBTreeMap::new();
        pending.insert((wallet_id, ACCOUNT, 0u32), addr);

        PlatformPaymentAddressProvider {
            wallet_manager,
            per_wallet,
            per_wallet_in_sync: BTreeMap::new(),
            pending,
            sync_height: 0,
            sync_timestamp: 0,
            last_known_recent_block: 0,
        }
    }

    /// Like [`provider_tracking_address`] but with an empty wallet
    /// manager — `sync_finished` and `current_balances` only touch the
    /// in-memory `per_wallet` / `per_wallet_in_sync` maps.
    fn provider_with_one_funded_address(
        addr: PlatformP2PKHAddress,
        f: AddressFunds,
    ) -> PlatformPaymentAddressProvider {
        let wallet_manager = Arc::new(RwLock::new(WalletManager::<PlatformWalletInfo>::new(
            Network::Testnet,
        )));
        provider_tracking_address(wallet_manager, WALLET, addr, f)
    }

    /// Stage `addr` as absent in the in-sync scratch — the shape
    /// `on_address_absent` produces (without the wallet-manager write,
    /// which needs a registered managed account).
    fn stage_absent(provider: &mut PlatformPaymentAddressProvider, addr: PlatformP2PKHAddress) {
        provider
            .per_wallet_in_sync
            .entry(WALLET)
            .or_default()
            .entry(ACCOUNT)
            .or_default()
            .absent
            .insert(addr);
    }

    /// Stage `addr` as found with `f` in the in-sync scratch — the shape
    /// `on_address_found` produces (without the wallet-manager write).
    fn stage_found(
        provider: &mut PlatformPaymentAddressProvider,
        addr: PlatformP2PKHAddress,
        f: AddressFunds,
    ) {
        provider
            .per_wallet_in_sync
            .entry(WALLET)
            .or_default()
            .entry(ACCOUNT)
            .or_default()
            .found
            .insert(addr, f);
    }

    /// `sync_finished`'s scratch merge applies height-pin freshness: a
    /// pass that ran against a lagging node stages a stale-but-valid
    /// absolute (older pin) for a row a fresher reconcile committed —
    /// the committed entry must survive, mirroring the persist diff's
    /// guard so memory and disk agree. A same-or-newer pin still lands.
    #[tokio::test]
    async fn sync_finished_keeps_fresher_pinned_committed_entry() {
        let addr = p2pkh(1);
        // Committed by the reconcile seam at the funding proof height.
        let mut provider =
            provider_with_one_funded_address(addr, funds_at(9_985_071_720, 0, 379_731));

        // A lagging pass stages the pre-funding absolute at an older pin.
        stage_found(&mut provider, addr, funds_at(0, 0, 379_728));
        provider.sync_finished().await;

        let seed: Vec<_> = provider.current_balances().collect();
        assert_eq!(
            seed[0].2,
            funds_at(9_985_071_720, 0, 379_731),
            "a stale-pinned scratch entry must not clobber a fresher \
             committed row"
        );

        // A genuinely newer pass replaces it.
        stage_found(&mut provider, addr, funds_at(5, 1, 379_740));
        provider.sync_finished().await;
        let seed: Vec<_> = provider.current_balances().collect();
        assert_eq!(seed[0].2, funds_at(5, 1, 379_740));
    }

    /// `sync_finished` must drop an address proven absent this pass from
    /// the committed `found` map so it stops seeding the next pass and
    /// `current_balances()` no longer yields it. This is the core of the
    /// stale-balance-after-chain-reset fix.
    #[tokio::test]
    async fn sync_finished_removes_absent_from_committed_found() {
        let addr = p2pkh(1);
        let mut provider = provider_with_one_funded_address(addr, funds(294_627_247_940, 5));

        // Sanity: before the pass, the funded address is part of the seed.
        let before: Vec<_> = provider.current_balances().collect();
        assert_eq!(before.len(), 1);
        assert_eq!(before[0].1, addr);

        // Prove it absent and flush.
        stage_absent(&mut provider, addr);
        provider.sync_finished().await;

        // The committed `found` no longer contains it; the next-pass seed
        // is empty.
        let after: Vec<_> = provider.current_balances().collect();
        assert!(
            after.is_empty(),
            "absent address must be removed from current_balances seed"
        );

        let account_state = provider
            .per_wallet
            .get(&WALLET)
            .and_then(|s| s.get(&ACCOUNT))
            .expect("account state present");
        assert!(
            !account_state.found().contains_key(&addr),
            "absent address must be removed from committed found map"
        );
        // The point-in-time absent set carries it for this pass.
        assert!(account_state.absent.contains(&addr));
    }

    /// A found address in the same pass survives; only the absent one is
    /// dropped. Found and absent are disjoint per pass — this pins that
    /// the removal doesn't clobber unrelated funded entries.
    #[tokio::test]
    async fn sync_finished_keeps_found_drops_absent() {
        let kept = p2pkh(1);
        let dropped = p2pkh(2);
        let mut provider = provider_with_one_funded_address(kept, funds(100, 1));

        // Register a second address (index 1) as tracked + funded so it's
        // part of the committed seed alongside the one that goes absent.
        if let Some(account_state) = provider
            .per_wallet
            .get_mut(&WALLET)
            .and_then(|s| s.get_mut(&ACCOUNT))
        {
            account_state.insert_persisted_entry(1, dropped, funds(999, 3));
        }

        // Stage a fresh balance for `kept` via the found-scratch and mark
        // `dropped` absent.
        provider
            .per_wallet_in_sync
            .entry(WALLET)
            .or_default()
            .entry(ACCOUNT)
            .or_default()
            .found
            .insert(kept, funds(150, 2));
        stage_absent(&mut provider, dropped);

        provider.sync_finished().await;

        let account_state = provider
            .per_wallet
            .get(&WALLET)
            .and_then(|s| s.get(&ACCOUNT))
            .expect("account state present");
        // `kept` survives with its refreshed funds; `dropped` is gone.
        assert_eq!(account_state.found().get(&kept), Some(&funds(150, 2)));
        assert!(!account_state.found().contains_key(&dropped));
    }

    /// An address can land in BOTH per-pass sets: proven absent at the
    /// full-scan checkpoint, then re-found by incremental catch-up. The
    /// find reflects the chain tip, so it must win — the committed
    /// `found` keeps the fresh funds and the committed `absent` must not
    /// list the address.
    #[tokio::test]
    async fn sync_finished_found_wins_over_absent_for_same_address() {
        let addr = p2pkh(1);
        let mut provider = provider_with_one_funded_address(addr, funds(100, 1));

        // Checkpoint proves it absent, catch-up re-finds it funded.
        stage_absent(&mut provider, addr);
        provider
            .per_wallet_in_sync
            .entry(WALLET)
            .or_default()
            .entry(ACCOUNT)
            .or_default()
            .found
            .insert(addr, funds(250, 2));

        provider.sync_finished().await;

        let account_state = provider
            .per_wallet
            .get(&WALLET)
            .and_then(|s| s.get(&ACCOUNT))
            .expect("account state present");
        assert_eq!(
            account_state.found().get(&addr),
            Some(&funds(250, 2)),
            "catch-up find must survive the checkpoint absent proof"
        );
        assert!(
            !account_state.absent.contains(&addr),
            "an address found at tip must not be committed as absent"
        );
        let seed: Vec<_> = provider.current_balances().collect();
        assert_eq!(seed.len(), 1);
        assert_eq!(seed[0].2, funds(250, 2));
    }

    /// Scratch staged by an aborted pass (the SDK only calls
    /// `sync_finished` on success) must not leak into the next pass —
    /// `prepare_for_sync` clears it. Without the clear, a stale staged
    /// absent would remove a committed balance the new pass never
    /// re-proved absent.
    #[tokio::test]
    async fn prepare_for_sync_clears_stale_scratch() {
        let addr = p2pkh(1);
        let mut provider = provider_with_one_funded_address(addr, funds(100, 1));

        // Simulate a pass that staged an absent proof and then aborted
        // before `sync_finished`.
        stage_absent(&mut provider, addr);
        assert!(!provider.per_wallet_in_sync.is_empty());

        provider.prepare_for_sync().await.expect("prepare");
        assert!(
            provider.per_wallet_in_sync.is_empty(),
            "aborted-pass scratch must be dropped before a new pass"
        );

        // The next successful pass with no absent proof keeps the
        // committed balance intact.
        provider.sync_finished().await;
        let seed: Vec<_> = provider.current_balances().collect();
        assert_eq!(seed.len(), 1);
        assert_eq!(seed[0].2, funds(100, 1));
    }

    /// End-to-end `on_address_absent` against a real wallet manager: the
    /// managed platform account's in-memory credit balance must be
    /// zeroed, since that map is what spend paths (identity-creation
    /// funding enumeration) read.
    #[tokio::test]
    async fn on_address_absent_zeroes_managed_account_balance() {
        use key_wallet::bip32::{ChildNumber, DerivationPath};
        use key_wallet::managed_account::address_pool::{AddressPool, AddressPoolType};
        use key_wallet::managed_account::managed_platform_account::ManagedPlatformAccount;
        use key_wallet::wallet::initialization::WalletAccountCreationOptions;

        let addr = p2pkh(1);
        let stale_balance = 294_627_247_940u64;

        // Real wallet manager with a registered wallet carrying a
        // platform payment managed account whose in-memory balance holds
        // the stale value.
        let mut wm = WalletManager::<PlatformWalletInfo>::new(Network::Testnet);
        let wallet_id = wm
            .create_wallet_with_random_mnemonic(WalletAccountCreationOptions::None)
            .expect("create wallet");
        {
            let info = wm.get_wallet_info_mut(&wallet_id).expect("wallet info");
            let base_path = DerivationPath::from(vec![
                ChildNumber::from_hardened_idx(9).expect("purpose"),
                ChildNumber::from_hardened_idx(1).expect("coin type"),
                ChildNumber::from_hardened_idx(17).expect("feature"),
                ChildNumber::from_hardened_idx(0).expect("subfeature"),
                ChildNumber::from_hardened_idx(0).expect("account"),
            ]);
            let pool = AddressPool::new_without_generation(
                base_path,
                AddressPoolType::Absent,
                20,
                Network::Testnet,
            );
            let platform_account = ManagedPlatformAccount::new(0, 0, pool, false);
            info.core_wallet
                .accounts
                .insert_platform_account(platform_account);
            let account = info
                .core_wallet
                .platform_payment_managed_account_at_index_mut(ACCOUNT)
                .expect("platform payment account");
            account.set_address_credit_balance(addr, stale_balance, None);
            assert_eq!(account.address_credit_balance(&addr), stale_balance);
        }
        let wallet_manager = Arc::new(RwLock::new(wm));

        let mut provider = provider_tracking_address(
            wallet_manager.clone(),
            wallet_id,
            addr,
            funds(stale_balance, 5),
        );

        provider
            .on_address_absent((wallet_id, ACCOUNT, 0), &addr)
            .await;

        // The absent proof is staged in the per-pass scratch...
        let staged = provider
            .per_wallet_in_sync
            .get(&wallet_id)
            .and_then(|s| s.get(&ACCOUNT))
            .expect("scratch staged");
        assert!(staged.absent.contains(&addr));

        // ...and the managed account's in-memory balance is zeroed.
        let wm = wallet_manager.read().await;
        let account = wm
            .get_wallet_info(&wallet_id)
            .expect("wallet info")
            .core_wallet
            .platform_payment_managed_account_at_index(ACCOUNT)
            .expect("platform payment account");
        assert_eq!(
            account.address_credit_balance(&addr),
            0,
            "on_address_absent must zero the in-memory managed-account balance"
        );
    }

    /// Records every changeset handed to `store`, so a test can assert what
    /// the reconciliation actually persisted.
    #[derive(Default)]
    struct CapturingPersister {
        stored: std::sync::Mutex<Vec<crate::changeset::PlatformWalletChangeSet>>,
    }

    impl crate::changeset::PlatformWalletPersistence for CapturingPersister {
        fn store(
            &self,
            _wallet_id: WalletId,
            changeset: crate::changeset::PlatformWalletChangeSet,
        ) -> Result<(), crate::changeset::PersistenceError> {
            self.stored.lock().expect("persister mutex").push(changeset);
            Ok(())
        }

        fn flush(&self, _wallet_id: WalletId) -> Result<(), crate::changeset::PersistenceError> {
            Ok(())
        }

        fn load(
            &self,
        ) -> Result<crate::changeset::ClientStartState, crate::changeset::PersistenceError>
        {
            Ok(crate::changeset::ClientStartState::default())
        }
    }

    /// Wallet wired to a capturing persister — the shared fixture for the
    /// reconciliation-seam tests below. `reconcile_address_infos` only
    /// touches provider / wallet_manager / persister; the rest mirrors the
    /// short-circuit fixture.
    async fn reconcile_seam_wallet(
        recorder: Arc<CapturingPersister>,
    ) -> (
        crate::wallet::platform_addresses::PlatformAddressWallet,
        Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    ) {
        use crate::broadcaster::SpvBroadcaster;
        use crate::events::PlatformEventManager;
        use crate::spv::SpvRuntime;
        use crate::wallet::asset_lock::manager::AssetLockManager;
        use crate::wallet::persister::WalletPersister;
        use crate::wallet::platform_addresses::PlatformAddressWallet;
        use tokio::sync::Notify;

        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let wallet_manager = Arc::new(RwLock::new(WalletManager::new(sdk.network)));
        let persister = WalletPersister::new(WALLET, recorder);
        let event_manager = Arc::new(PlatformEventManager::new(Vec::new()));
        let spv = Arc::new(SpvRuntime::new(Arc::clone(&wallet_manager), event_manager));
        let broadcaster = Arc::new(SpvBroadcaster::new(spv));
        let asset_locks = Arc::new(AssetLockManager::new(
            Arc::clone(&sdk),
            Arc::clone(&wallet_manager),
            WALLET,
            Arc::new(Notify::new()),
            broadcaster,
            persister.clone(),
        ));
        let wallet = PlatformAddressWallet::new(
            sdk,
            Arc::clone(&wallet_manager),
            WALLET,
            asset_locks,
            persister,
        );
        (wallet, wallet_manager)
    }

    /// Integration regression for the reconciliation *contract* — not
    /// just the pure entry builder. `reconcile_address_infos` must build
    /// AND **persist** a `PlatformAddressChangeSet` carrying the proof's
    /// post-spend balance for a spent address resolved via the provider's
    /// persisted state. The reported bug was the *missing persist* (the SDK's
    /// `address_infos` were discarded), so this pins that `store` actually
    /// fires with the decremented entry — a helper-only test would still pass
    /// if `reconcile_address_infos` stopped persisting.
    #[tokio::test]
    async fn reconcile_address_infos_persists_decremented_balance() {
        use dash_sdk::query_types::AddressInfo;

        let recorder = Arc::new(CapturingPersister::default());
        let (wallet, wallet_manager) = reconcile_seam_wallet(recorder.clone()).await;

        // The provider knows the spent address via its persisted bijection
        // (pre-spend balance 100).
        let addr = p2pkh(0x11);
        let provider =
            provider_tracking_address(Arc::clone(&wallet_manager), WALLET, addr, funds(100, 1));
        *wallet.provider.write().await = Some(provider);

        // The top-up spent it; the proof attests post-spend balance 5, nonce 4.
        let spent = PlatformAddress::P2pkh([0x11; 20]);
        let mut address_infos = AddressInfos::new();
        address_infos.insert(
            spent,
            Some(AddressInfo {
                address: spent,
                nonce: 4,
                balance: 5,
            }),
        );

        wallet
            .reconcile_address_infos(&address_infos, 42, "test top-up")
            .await;

        // The reconciliation must have PERSISTED the decremented entry — the
        // contract the original bug broke by discarding `address_infos`.
        // Scope the std mutex guard so it isn't held across the provider
        // read await below.
        {
            let stored = recorder.stored.lock().expect("persister mutex");
            let entry = stored
                .iter()
                .filter_map(|cs| cs.platform_addresses.as_ref())
                .flat_map(|pa| pa.addresses.iter())
                .find(|e| e.address == addr)
                .expect("a persisted platform-address entry for the spent address");
            assert_eq!(entry.account_index, ACCOUNT);
            assert_eq!(
                entry.funds.balance, 5,
                "persists the proof's post-spend balance, not the stale pre-spend value"
            );
            assert_eq!(entry.funds.nonce, 4, "persists the bumped nonce");
        }

        // ...and the provider's committed `found` seed — the sync-diff
        // baseline and the seed `current_balances()` hands the SDK — must
        // agree with what was just applied, so reconciliation and the
        // background sync stop diverging.
        let guard = wallet.provider.read().await;
        let seed: Vec<_> = guard
            .as_ref()
            .expect("provider present")
            .current_balances()
            .collect();
        assert_eq!(seed.len(), 1);
        assert_eq!(
            seed[0].2,
            funds_at(5, 4, 42),
            "committed found seed must carry the reconciled funds pinned \
             at the proof height"
        );
    }

    /// ADDR-09, credit side: a committed credit is pinned at the proof
    /// height (`AddressFunds::as_of_height`), which is what stops the
    /// sync's delta replay from re-applying the on-chain `AddToCredits`
    /// on top of the just-committed absolute — so the seam no longer
    /// needs to invalidate the incremental watermark (the old gate,
    /// which forced a full rescan yet could not protect the rescan
    /// itself from the same replay). The fast incremental cadence is
    /// preserved for every flow.
    #[tokio::test]
    async fn reconcile_pins_committed_credit_and_keeps_watermark() {
        use dash_sdk::query_types::AddressInfo;

        let recorder = Arc::new(CapturingPersister::default());
        let (wallet, wallet_manager) = reconcile_seam_wallet(recorder).await;

        // Mid-incremental-sync provider: non-zero watermark, pre-credit
        // seed balance 100 at nonce 1.
        let addr = p2pkh(0x11);
        let mut provider =
            provider_tracking_address(Arc::clone(&wallet_manager), WALLET, addr, funds(100, 1));
        provider.set_stored_sync_state(10, 20, 30);
        *wallet.provider.write().await = Some(provider);

        // A transition credited the address (delta on-chain); the proof
        // attests the post-credit ABSOLUTE balance 600. Output credits
        // leave the address nonce untouched, so it stays at 1.
        let credited = PlatformAddress::P2pkh([0x11; 20]);
        let mut address_infos = AddressInfos::new();
        address_infos.insert(
            credited,
            Some(AddressInfo {
                address: credited,
                nonce: 1,
                balance: 600,
            }),
        );
        let cs = wallet
            .reconcile_address_infos(&address_infos, 42, "test credit")
            .await;
        assert_eq!(cs.addresses.len(), 1, "the credit must be committed");
        assert_eq!(
            cs.addresses[0].funds,
            funds_at(600, 1, 42),
            "the persisted entry must carry the proof-height pin — the \
             sync's replay gate reads it to drop the on-chain credit delta"
        );

        let guard = wallet.provider.read().await;
        let provider = guard.as_ref().expect("provider present");
        assert_eq!(
            provider.last_sync_timestamp(),
            Some(20),
            "a committed credit must keep the incremental watermark — the \
             pin, not a forced full rescan, is what prevents the ADDR-09 \
             double-count"
        );
        assert_eq!(provider.last_sync_height(), 10);
        assert_eq!(provider.last_known_recent_block(), 30);
        let seed: Vec<_> = provider.current_balances().collect();
        assert_eq!(seed.len(), 1);
        assert_eq!(
            seed[0].2,
            funds_at(600, 1, 42),
            "the committed found seed must carry the pinned funds"
        );
    }

    /// Drain side: an input-only reconciliation (e.g. an
    /// external-recipient transfer or a withdrawal) keeps the incremental
    /// watermark, exactly as before the pin existed. Inputs are recorded
    /// on-chain as absolute `SetBalanceToAddress` ops; the pin makes them
    /// (and any future change output) replay-safe without touching the
    /// sync cadence.
    #[tokio::test]
    async fn reconcile_keeps_watermark_on_input_only_drain() {
        use dash_sdk::query_types::AddressInfo;

        let recorder = Arc::new(CapturingPersister::default());
        let (wallet, wallet_manager) = reconcile_seam_wallet(recorder).await;

        let addr = p2pkh(0x11);
        let mut provider =
            provider_tracking_address(Arc::clone(&wallet_manager), WALLET, addr, funds(100, 1));
        provider.set_stored_sync_state(10, 20, 30);
        *wallet.provider.write().await = Some(provider);

        // A spend drained the address: absolute post-spend balance 5,
        // bumped input nonce 4. No credited outputs declared.
        let spent = PlatformAddress::P2pkh([0x11; 20]);
        let mut address_infos = AddressInfos::new();
        address_infos.insert(
            spent,
            Some(AddressInfo {
                address: spent,
                nonce: 4,
                balance: 5,
            }),
        );

        let cs = wallet
            .reconcile_address_infos(&address_infos, 42, "test drain")
            .await;
        assert_eq!(cs.addresses.len(), 1, "the drain must be committed");

        let guard = wallet.provider.read().await;
        let provider = guard.as_ref().expect("provider present");
        assert_eq!(
            provider.last_sync_timestamp(),
            Some(20),
            "input-only reconciliation must keep the incremental watermark"
        );
        assert_eq!(provider.last_sync_height(), 10);
        assert_eq!(provider.last_known_recent_block(), 30);
    }

    /// No-op skip: a proof entry identical to the committed seed —
    /// including the pin — is dropped (`unchanged_skipped`) instead of
    /// re-committed, avoiding persister churn when a background sync
    /// already applied this credit at the same height.
    #[tokio::test]
    async fn reconcile_skips_entry_identical_to_committed_seed() {
        use dash_sdk::query_types::AddressInfo;

        let recorder = Arc::new(CapturingPersister::default());
        let (wallet, wallet_manager) = reconcile_seam_wallet(recorder).await;

        // Seed already carries the post-credit state (600, nonce 1)
        // pinned at the same height this reconcile will use — the
        // background sync applied the credit before this reconcile ran.
        let addr = p2pkh(0x11);
        let mut provider = provider_tracking_address(
            Arc::clone(&wallet_manager),
            WALLET,
            addr,
            funds_at(600, 1, 42),
        );
        provider.set_stored_sync_state(10, 20, 30);
        *wallet.provider.write().await = Some(provider);

        let credited = PlatformAddress::P2pkh([0x11; 20]);
        let mut address_infos = AddressInfos::new();
        address_infos.insert(
            credited,
            Some(AddressInfo {
                address: credited,
                nonce: 1,
                balance: 600,
            }),
        );
        let cs = wallet
            .reconcile_address_infos(&address_infos, 42, "test unchanged")
            .await;
        assert!(
            cs.addresses.is_empty(),
            "unchanged entry must be skipped, not re-committed"
        );

        let guard = wallet.provider.read().await;
        let provider = guard.as_ref().expect("provider present");
        assert_eq!(
            provider.last_sync_timestamp(),
            Some(20),
            "a no-op reconcile must leave the sync watermark untouched"
        );
    }

    /// Freshness guard: an entry whose nonce is below the committed seed's
    /// (a background sync — or a later transition — already committed
    /// fresher state) must be dropped, not applied over the fresher value.
    #[test]
    fn commit_reconciliation_drops_stale_nonce() {
        use dash_sdk::query_types::AddressInfo;

        let addr = p2pkh(0x11);
        // Committed seed already at nonce 5 (e.g. the 15s background sync
        // landed a fresher state while the proof was in flight).
        let mut provider = provider_with_one_funded_address(addr, funds(700, 5));

        let spent = PlatformAddress::P2pkh([0x11; 20]);
        let mut address_infos = AddressInfos::new();
        address_infos.insert(
            spent,
            Some(AddressInfo {
                address: spent,
                nonce: 3,
                balance: 100,
            }),
        );

        let outcome = provider.commit_reconciliation(&WALLET, &address_infos, &BTreeMap::new(), 0);

        assert_eq!(outcome.resolved, 1);
        assert_eq!(outcome.stale_skipped, 1);
        assert!(
            outcome.entries.is_empty(),
            "stale entry must not be applied"
        );
        // The fresher committed funds survive.
        let seed: Vec<_> = provider.current_balances().collect();
        assert_eq!(seed.len(), 1);
        assert_eq!(seed[0].2, funds(700, 5));
    }

    /// A zero-funds removal that pool-resolves to an `address_index`
    /// already paired to a DIFFERENT address in the bijection must still
    /// zero the in-memory `found` row AND be emitted downstream —
    /// otherwise a durable persister row for the removed address would
    /// resurrect after restart. The bijection stays untouched so the
    /// pre-existing `(index -> other addr)` pairing isn't evicted.
    #[test]
    fn commit_reconciliation_index_conflict_still_emits_removal() {
        let owned = p2pkh(0x11);
        let conflicting = p2pkh(0x77);
        let mut provider = provider_with_one_funded_address(owned, funds(700, 3));
        {
            let state = provider
                .per_wallet
                .get_mut(&WALLET)
                .and_then(|s| s.get_mut(&ACCOUNT))
                .expect("account state present");
            // Pin `conflicting` at index 5 with a balance we must protect.
            state.insert_persisted_entry(5, conflicting, funds(200, 1));
            // Stale `found` row for the address that will be removed,
            // seeded at the SAME index 5 to force the conflict.
            state.found.insert(p2pkh(0x22), funds(999, 4));
        }

        // The removed address pool-resolves to index 5 — a DIFFERENT
        // address than the bijection holds there.
        let removed = p2pkh(0x22);
        let removed_addr = PlatformAddress::P2pkh([0x22; 20]);
        let mut pool_indexes = BTreeMap::new();
        pool_indexes.insert(removed, (ACCOUNT, 5u32));

        let mut address_infos = AddressInfos::new();
        // Fully-consumed input: Drive elides the info → removal entry.
        address_infos.insert(removed_addr, None);

        let outcome = provider.commit_reconciliation(&WALLET, &address_infos, &pool_indexes, 42);

        // The removal is emitted so the durable persister writes the zero.
        assert_eq!(outcome.entries.len(), 1, "removal survives the guard");
        assert_eq!(outcome.entries[0].address, removed);
        assert_eq!(outcome.entries[0].funds, funds_at(0, 0, 42));

        let state = provider
            .per_wallet
            .get(&WALLET)
            .and_then(|s| s.get(&ACCOUNT))
            .expect("account state present");
        // In-memory `found` for the removed address is dropped.
        assert!(!state.found.contains_key(&removed));
        // The bijection is unchanged — pre-existing pairing survives.
        assert_eq!(
            state.addresses.get_by_left(&5u32).copied(),
            Some(conflicting)
        );
        assert!(state.addresses.get_by_right(&removed).is_none());
        // The protected address's balance is untouched.
        assert_eq!(state.found.get(&conflicting).copied(), Some(funds(200, 1)));
    }

    /// A CREDIT (non-zero funds) that pool-resolves to an already-taken
    /// derivation index must be dropped outright: neither applied to
    /// `found` nor emitted, and the bijection untouched. Committing it
    /// would either evict the existing pairing or persist a seed
    /// `current_balances` can't round-trip.
    #[test]
    fn commit_reconciliation_index_conflict_drops_credit() {
        use dash_sdk::query_types::AddressInfo;

        let owned = p2pkh(0x11);
        let conflicting = p2pkh(0x77);
        let mut provider = provider_with_one_funded_address(owned, funds(700, 3));
        {
            let state = provider
                .per_wallet
                .get_mut(&WALLET)
                .and_then(|s| s.get_mut(&ACCOUNT))
                .expect("account state present");
            state.insert_persisted_entry(5, conflicting, funds(200, 1));
        }

        // A credit for a fresh address that pool-resolves to the taken
        // index 5.
        let credited = p2pkh(0x33);
        let credited_addr = PlatformAddress::P2pkh([0x33; 20]);
        let mut pool_indexes = BTreeMap::new();
        pool_indexes.insert(credited, (ACCOUNT, 5u32));

        let mut address_infos = AddressInfos::new();
        address_infos.insert(
            credited_addr,
            Some(AddressInfo {
                address: credited_addr,
                nonce: 2,
                balance: 5_000,
            }),
        );

        let outcome = provider.commit_reconciliation(&WALLET, &address_infos, &pool_indexes, 42);

        assert!(
            outcome.entries.is_empty(),
            "a credit under a conflicting index is dropped, not emitted"
        );
        let state = provider
            .per_wallet
            .get(&WALLET)
            .and_then(|s| s.get(&ACCOUNT))
            .expect("account state present");
        // `found` never gained the conflicting credit.
        assert!(!state.found.contains_key(&credited));
        // Bijection untouched: index 5 still → conflicting, and the
        // credited address was not inserted.
        assert_eq!(
            state.addresses.get_by_left(&5u32).copied(),
            Some(conflicting)
        );
        assert!(state.addresses.get_by_right(&credited).is_none());
        assert_eq!(state.found.get(&conflicting).copied(), Some(funds(200, 1)));
    }

    /// Restore-side guard for the index-conflicting removal. The write
    /// side is [`commit_reconciliation`](PlatformPaymentAddressProvider::commit_reconciliation):
    /// when a reconcile removal pool-resolves to an index already owned by
    /// a different, funded address it leaves the in-memory bijection
    /// untouched but still emits the zero so the balance can't resurrect —
    /// which persists a zeroed row that can collide with the funded row on
    /// disk. A durable store can therefore hold a funded row and a zeroed
    /// *removal remnant* that both claim the same derivation index. On
    /// restart the rows load in arbitrary fetch order and seed the account
    /// bijection one at a time; a naive `BiBTreeMap::insert` would let the
    /// zero remnant evict the funded `(index -> address)` pairing,
    /// orphaning its balance from `current_balances` for one of the two
    /// orders. `insert_persisted_entry` must land on the same correct state
    /// in BOTH orders: the funded pairing survives, so the balance the
    /// engine reads back (the intersection of `found` and the bijection,
    /// i.e. what `current_balances` yields) is exactly the funded one — the
    /// remnant's inert zero `found` row is never paired, so it can't be
    /// yielded.
    #[test]
    fn insert_persisted_entry_removal_remnant_never_orphans_funded_pairing() {
        let funded = p2pkh(0x77);
        let remnant = p2pkh(0x22);
        const INDEX: AddressIndex = 5;

        for funded_first in [true, false] {
            let mut state = PerAccountPlatformAddressState::from_persisted(
                test_xpub(),
                BiBTreeMap::new(),
                BTreeMap::new(),
            );

            // Same two durable rows, opposite restore (fetch) order.
            if funded_first {
                state.insert_persisted_entry(INDEX, funded, funds(200, 3));
                state.insert_persisted_entry(INDEX, remnant, funds(0, 0));
            } else {
                state.insert_persisted_entry(INDEX, remnant, funds(0, 0));
                state.insert_persisted_entry(INDEX, funded, funds(200, 3));
            }

            // The funded pairing survives in the bijection...
            assert_eq!(
                state.addresses.get_by_left(&INDEX).copied(),
                Some(funded),
                "funded (index -> address) pairing must survive (funded_first={funded_first})"
            );
            // ...and the zero remnant never holds the slot it would have to
            // evict the funded pairing to take.
            assert!(
                state.addresses.get_by_right(&remnant).is_none(),
                "removal remnant must not hold a bijection slot (funded_first={funded_first})"
            );

            // What the engine actually reads back is the intersection of
            // `found` and the bijection — precisely `current_balances`. It
            // must be exactly the funded balance; the remnant's inert zero
            // `found` row is unpaired and therefore never yielded.
            let restored: Vec<_> = state
                .found
                .iter()
                .filter_map(|(addr, &f)| state.addresses.get_by_right(addr).map(|_| (*addr, f)))
                .collect();
            assert_eq!(
                restored,
                vec![(funded, funds(200, 3))],
                "only the funded balance may round-trip (funded_first={funded_first})"
            );
        }
    }

    /// A zero-balance/zero-nonce row is indistinguishable from a legitimate
    /// freshly-derived, never-funded address (both are `{0, 0}`). With a
    /// free index it must restore *normally* — into both `found` and the
    /// bijection — so the guard against index-conflicting removals never
    /// swallows a real unfunded address. (The `platform-wallet-storage`
    /// SQLite reconstruction test pins the same expectation end to end.)
    #[test]
    fn insert_persisted_entry_unfunded_derived_address_restores_normally() {
        let unfunded = p2pkh(0x22);
        let mut state = PerAccountPlatformAddressState::from_persisted(
            test_xpub(),
            BiBTreeMap::new(),
            BTreeMap::new(),
        );

        state.insert_persisted_entry(7, unfunded, funds(0, 0));

        assert_eq!(
            state.found.get(&unfunded).copied(),
            Some(funds(0, 0)),
            "a never-funded derived address must still seed found (balance 0)"
        );
        assert_eq!(
            state.addresses.get_by_left(&7u32).copied(),
            Some(unfunded),
            "a free-slot row extends the bijection normally"
        );
    }

    /// An entry identical to the committed seed is a no-op and is dropped
    /// to avoid persister churn.
    #[test]
    fn commit_reconciliation_drops_unchanged_entry() {
        use dash_sdk::query_types::AddressInfo;

        let addr = p2pkh(0x11);
        let mut provider = provider_with_one_funded_address(addr, funds(700, 5));

        let same = PlatformAddress::P2pkh([0x11; 20]);
        let mut address_infos = AddressInfos::new();
        address_infos.insert(
            same,
            Some(AddressInfo {
                address: same,
                nonce: 5,
                balance: 700,
            }),
        );

        let outcome = provider.commit_reconciliation(&WALLET, &address_infos, &BTreeMap::new(), 0);

        assert_eq!(outcome.resolved, 1);
        assert_eq!(outcome.unchanged_skipped, 1);
        assert!(outcome.entries.is_empty());
    }

    /// A fresh entry (nonce at or above the seed's) is applied and
    /// committed to `found`, replacing the older funds.
    #[test]
    fn commit_reconciliation_commits_fresh_entry_to_found() {
        use dash_sdk::query_types::AddressInfo;

        let addr = p2pkh(0x11);
        let mut provider = provider_with_one_funded_address(addr, funds(700, 3));

        let spent = PlatformAddress::P2pkh([0x11; 20]);
        let mut address_infos = AddressInfos::new();
        address_infos.insert(
            spent,
            Some(AddressInfo {
                address: spent,
                nonce: 4,
                balance: 50,
            }),
        );

        let outcome = provider.commit_reconciliation(&WALLET, &address_infos, &BTreeMap::new(), 0);

        assert_eq!(outcome.entries.len(), 1);
        assert_eq!(outcome.entries[0].funds, funds(50, 4));
        assert_eq!(outcome.entries[0].address_index, 0);
        let seed: Vec<_> = provider.current_balances().collect();
        assert_eq!(seed.len(), 1);
        assert_eq!(seed[0].2, funds(50, 4));
    }

    /// Zero funds (balance 0, nonce 0) means the address was removed from
    /// Platform state (fully consumed input). Pinned at a height above the
    /// committed seed's, the removal wins by height authority (nonces
    /// cannot order a "gone" state) and drops the address from the
    /// committed `found` seed, mirroring the sync's absent handling.
    #[test]
    fn commit_reconciliation_zero_funds_removes_from_found() {
        let addr = p2pkh(0x11);
        let mut provider = provider_with_one_funded_address(addr, funds(700, 5));

        let consumed = PlatformAddress::P2pkh([0x11; 20]);
        let mut address_infos = AddressInfos::new();
        // Drive elides the info for a fully consumed input.
        address_infos.insert(consumed, None);

        let outcome = provider.commit_reconciliation(&WALLET, &address_infos, &BTreeMap::new(), 42);

        assert_eq!(outcome.entries.len(), 1);
        assert_eq!(outcome.entries[0].funds, funds_at(0, 0, 42));
        assert!(
            provider.current_balances().next().is_none(),
            "consumed address must be dropped from the found seed"
        );
    }

    /// The live-pool fallback must NOT resolve addresses belonging to an
    /// account the provider doesn't track: there is no per-account state
    /// to commit the funds into, so emitting the entry would violate the
    /// contract that every emitted entry is committed to the `found` seed
    /// (the applied balance would silently diverge from the sync-diff
    /// baseline).
    #[test]
    fn commit_reconciliation_pool_fallback_skips_untracked_account() {
        use dash_sdk::query_types::AddressInfo;

        let known = p2pkh(0x11);
        let mut provider = provider_with_one_funded_address(known, funds(700, 3));

        // Live-pool address on an account (7) the provider has no state for.
        let untracked = p2pkh(0x33);
        let mut pool_indexes = BTreeMap::new();
        pool_indexes.insert(untracked, (7u32, 0u32));

        let untracked_addr = PlatformAddress::P2pkh([0x33; 20]);
        let mut address_infos = AddressInfos::new();
        address_infos.insert(
            untracked_addr,
            Some(AddressInfo {
                address: untracked_addr,
                nonce: 0,
                balance: 9_999,
            }),
        );

        let outcome = provider.commit_reconciliation(&WALLET, &address_infos, &pool_indexes, 42);

        assert_eq!(
            outcome.resolved, 0,
            "an untracked account's pool address must stay unresolved"
        );
        assert!(outcome.entries.is_empty());
    }

    /// Height authority: an absolute pinned at a LATER height is
    /// authoritative even when it revises the balance DOWNWARD — this is
    /// the ADDR-09 healing property. A poisoned legacy row (e.g. a
    /// double-counted balance persisted before the pin existed, pin 0)
    /// must yield to a proof-attested absolute at any real height.
    #[test]
    fn commit_reconciliation_later_pin_revises_balance_downward() {
        use dash_sdk::query_types::AddressInfo;

        let addr = p2pkh(0x11);
        // Poisoned pre-pin seed: 2X with "unknown provenance" (pin 0).
        let mut provider = provider_with_one_funded_address(addr, funds(19_970_143_440, 0));

        let credited = PlatformAddress::P2pkh([0x11; 20]);
        let mut address_infos = AddressInfos::new();
        address_infos.insert(
            credited,
            Some(AddressInfo {
                address: credited,
                nonce: 0,
                balance: 9_985_071_720,
            }),
        );

        let outcome =
            provider.commit_reconciliation(&WALLET, &address_infos, &BTreeMap::new(), 379_395);

        assert_eq!(outcome.entries.len(), 1, "the downward revision commits");
        assert_eq!(outcome.stale_skipped, 0);
        let seed: Vec<_> = provider.current_balances().collect();
        assert_eq!(
            seed[0].2,
            funds_at(9_985_071_720, 0, 379_395),
            "the later-pinned single-counted absolute replaces the 2x row"
        );
    }

    /// Height authority, stale side: an absolute pinned BELOW the
    /// committed seed's pin is stale — a sync pass or later transition
    /// already committed state attested at a later block — and must be
    /// dropped even though its nonce is not below the seed's (nonces
    /// cannot order receive-only states across blocks).
    #[test]
    fn commit_reconciliation_drops_stale_height() {
        use dash_sdk::query_types::AddressInfo;

        let addr = p2pkh(0x11);
        let mut provider = provider_with_one_funded_address(addr, funds_at(1_000, 0, 100));

        let credited = PlatformAddress::P2pkh([0x11; 20]);
        let mut address_infos = AddressInfos::new();
        address_infos.insert(
            credited,
            Some(AddressInfo {
                address: credited,
                nonce: 0,
                balance: 600,
            }),
        );

        let outcome = provider.commit_reconciliation(&WALLET, &address_infos, &BTreeMap::new(), 50);

        assert_eq!(outcome.stale_skipped, 1, "older-pinned absolute is stale");
        assert!(outcome.entries.is_empty());
        let seed: Vec<_> = provider.current_balances().collect();
        assert_eq!(
            seed[0].2,
            funds_at(1_000, 0, 100),
            "the fresher committed funds survive"
        );
    }

    /// An address missing from the provider bijection (derived since the
    /// last sync, e.g. a fresh change address) resolves through the
    /// live-pool fallback, and the pair is merged into the bijection so
    /// `current_balances` can yield its committed funds.
    #[test]
    fn commit_reconciliation_pool_fallback_extends_bijection() {
        use dash_sdk::query_types::AddressInfo;

        let known = p2pkh(0x11);
        let mut provider = provider_with_one_funded_address(known, funds(700, 3));

        // Fresh change address: NOT in the bijection, only in the live pool.
        let fresh = p2pkh(0x22);
        let mut pool_indexes = BTreeMap::new();
        pool_indexes.insert(fresh, (ACCOUNT, 9u32));

        let fresh_addr = PlatformAddress::P2pkh([0x22; 20]);
        let mut address_infos = AddressInfos::new();
        address_infos.insert(
            fresh_addr,
            Some(AddressInfo {
                address: fresh_addr,
                nonce: 0,
                balance: 1_234,
            }),
        );

        let outcome = provider.commit_reconciliation(&WALLET, &address_infos, &pool_indexes, 42);

        assert_eq!(outcome.entries.len(), 1);
        assert_eq!(
            outcome.entries[0].address_index, 9,
            "index resolved from the live-pool fallback"
        );
        // The bijection gained the pair, so the committed funds are part
        // of the next sync's seed.
        let seed: Vec<_> = provider.current_balances().collect();
        let fresh_row = seed
            .iter()
            .find(|(_, a, _)| *a == fresh)
            .expect("fresh address must appear in the found seed");
        assert_eq!(fresh_row.0, (WALLET, ACCOUNT, 9));
        assert_eq!(fresh_row.2, funds_at(1_234, 0, 42));
    }

    /// `reset_sync_state` must zero the incremental watermark AND drop
    /// the cached `found` seed, so the next pass is a full rescan rather
    /// than an incremental catch-up. This is the core of the platform
    /// "Clear" fix — without the seed drop, a non-empty `found` would
    /// re-seed the balances the next incremental round, and a non-zero
    /// `sync_timestamp` would keep the SDK out of full-scan mode.
    #[tokio::test]
    async fn reset_sync_state_clears_watermark_and_seed() {
        let addr = p2pkh(1);
        let mut provider = provider_with_one_funded_address(addr, funds(294_627_247_940, 5));

        // Simulate a wallet mid-incremental-sync: non-zero watermark and
        // a populated balance seed.
        provider.set_stored_sync_state(10, 20, 30);
        assert_eq!(provider.last_sync_height(), 10);
        assert_eq!(provider.last_sync_timestamp(), Some(20));
        assert_eq!(provider.last_known_recent_block(), 30);
        assert_eq!(provider.current_balances().count(), 1);

        provider.reset_sync_state();

        // Watermark fully zeroed → SDK drops back to full-scan mode
        // (`last_sync_timestamp() == None` is the full-scan trigger).
        assert_eq!(provider.last_sync_height(), 0);
        assert_eq!(provider.last_sync_timestamp(), None);
        assert_eq!(provider.last_known_recent_block(), 0);
        // Seed emptied → nothing re-seeds the next incremental pass.
        assert_eq!(
            provider.current_balances().count(),
            0,
            "reset must drop the cached `found` seed"
        );
    }
}
