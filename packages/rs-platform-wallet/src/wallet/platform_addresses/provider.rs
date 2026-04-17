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
pub(crate) struct PerAccountPlatformAddressState {
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
    #[allow(dead_code)]
    pub(crate) fn from_persisted(
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
}

/// Per-wallet account map — keys are DIP-17 account indexes (hardened
/// level), values carry the account-level state.
pub(crate) type PerWalletPlatformAddressState = BTreeMap<u32, PerAccountPlatformAddressState>;

/// Address provider covering every platform payment account across
/// every registered wallet.
///
/// Implements the SDK's [`AddressProvider`] trait by presenting a flat
/// view of pending addresses spanning all wallets.
pub(crate) struct PlatformPaymentAddressProvider {
    /// Shared wallet manager for gap-limit extension.
    wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    /// Per-wallet tracked state (everything except pending).
    per_wallet: BTreeMap<WalletId, PerWalletPlatformAddressState>,
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
            pending,
            sync_height: 0,
            sync_timestamp: 0,
            last_known_recent_block: 0,
        })
    }

    /// Rebuild a provider from already-populated per-wallet state and
    /// an incremental-sync watermark. Used on startup when a
    /// persister has state to restore — skips the live
    /// `AddressPool` scan that [`from_wallets`](Self::from_wallets)
    /// performs, because the caller's state is the source of truth.
    ///
    /// `pending` starts empty; the first
    /// [`prepare_for_sync`](Self::prepare_for_sync) call repopulates
    /// it from each account's `AddressPool` in the wallet manager.
    #[allow(dead_code)]
    pub(crate) fn from_persisted(
        wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        per_wallet: BTreeMap<WalletId, PerWalletPlatformAddressState>,
        sync_height: u64,
        sync_timestamp: u64,
        last_known_recent_block: u64,
    ) -> Self {
        Self {
            wallet_manager,
            per_wallet,
            pending: BiBTreeMap::new(),
            sync_height,
            sync_timestamp,
            last_known_recent_block,
        }
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
    /// from their respective `AddressPool`s and clear the per-pass
    /// `absent` set. `found` is intentionally preserved across syncs
    /// — it doubles as the `current_balances()` seed for the next
    /// incremental round.
    ///
    /// Call before each sync round.
    pub(crate) async fn prepare_for_sync(&mut self) -> Result<(), PlatformWalletError> {
        let wallet_ids: Vec<WalletId> = self.per_wallet.keys().copied().collect();

        // Refresh provider-level pending and, in the same pass, each
        // account's `addresses` bimap so both stay in sync with the
        // wallet's pool.
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
                    account_state.addresses.clear();
                    for (&index, addr_info) in &managed.addresses.addresses {
                        let Ok(p2pkh) = PlatformP2PKHAddress::from_address(&addr_info.address)
                        else {
                            continue;
                        };
                        out.insert((wallet_id, account_index, index), p2pkh);
                        account_state.addresses.insert(index, p2pkh);
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

        // Record in our per-account state first so a later error
        // walking the key-wallet pool doesn't lose the hit.
        let Some(key_source) = self
            .per_wallet
            .get_mut(&wallet_id)
            .and_then(|s| s.get_mut(&account_index))
            .map(|account_state| {
                account_state.addresses.insert(address_index, p2pkh);
                account_state.found.insert(p2pkh, funds);
                KeySource::Public(account_state.extended_public_key)
            })
        else {
            tracing::warn!(
                "on_address_found: no tracked state for wallet {} account {}",
                hex::encode(wallet_id),
                account_index
            );
            return;
        };

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

        if let Some(account_state) = self
            .per_wallet
            .get_mut(&wallet_id)
            .and_then(|s| s.get_mut(&account_index))
        {
            account_state.addresses.insert(address_index, p2pkh);
            account_state.absent.insert(p2pkh);
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
