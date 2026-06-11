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
use dash_sdk::platform::address_sync::{
    AddressFunds, AddressIndex, AddressProvider, AddressSyncResult,
};
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
    pub fn insert_persisted_entry(
        &mut self,
        address_index: AddressIndex,
        address: PlatformP2PKHAddress,
        funds: AddressFunds,
    ) {
        self.addresses.insert(address_index, address);
        self.found.insert(address, funds);
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
    /// read the value without going through the trait.
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
        // Extend-then-remove order: the SDK proves an address either
        // present or absent in a single pass, never both, so the two sets
        // are disjoint and the order is academic. We still extend first
        // and remove second so that, in the impossible-by-contract case
        // where an address showed up in both, the absent proof wins —
        // an address gone from state must not stay funded.
        let drained = std::mem::take(&mut self.per_wallet_in_sync);
        for (wallet_id, wallet_scratch) in drained {
            let Some(wallet_state) = self.per_wallet.get_mut(&wallet_id) else {
                continue;
            };
            for (account_index, account_scratch) in wallet_scratch {
                let Some(account_state) = wallet_state.get_mut(&account_index) else {
                    continue;
                };
                account_state.found.extend(account_scratch.found);
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
        AddressFunds { balance, nonce }
    }

    /// Build a provider whose committed `per_wallet` tracks a single
    /// account with one funded address (index 0). No wallet is registered
    /// in the manager — `sync_finished` and `current_balances` only touch
    /// the in-memory `per_wallet` / `per_wallet_in_sync` maps.
    fn provider_with_one_funded_address(
        addr: PlatformP2PKHAddress,
        f: AddressFunds,
    ) -> PlatformPaymentAddressProvider {
        let wallet_manager = Arc::new(RwLock::new(WalletManager::<PlatformWalletInfo>::new(
            Network::Testnet,
        )));

        let mut account_state = PerAccountPlatformAddressState::from_persisted(
            test_xpub(),
            BiBTreeMap::new(),
            BTreeMap::new(),
        );
        account_state.insert_persisted_entry(0, addr, f);

        let mut wallet_state = PerWalletPlatformAddressState::new();
        wallet_state.insert(ACCOUNT, account_state);

        let mut per_wallet = BTreeMap::new();
        per_wallet.insert(WALLET, wallet_state);

        let mut pending = BiBTreeMap::new();
        pending.insert((WALLET, ACCOUNT, 0u32), addr);

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
}
