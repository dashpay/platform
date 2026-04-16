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
use dpp::address_funds::PlatformAddress;
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
struct PerAccountPlatformAddressState {
    /// Public-key material for this account — the xpub that address
    /// derivation (gap-limit extension inside
    /// [`PlatformPaymentAddressProvider::on_address_found_in_pool`])
    /// needs to produce a [`KeySource::Public`] on demand.
    extended_public_key: ExtendedPubKey,
    /// Every address this account has derived, as a bijection between
    /// the DIP-17 derivation index and the P2PKH hash. Lets `found` /
    /// `absent` store bare `PlatformP2PKHAddress` values and still
    /// recover the index when building `current_balances` tuples.
    addresses: BiBTreeMap<AddressIndex, PlatformP2PKHAddress>,
    /// Addresses proven present with their balance.
    found: BTreeMap<PlatformP2PKHAddress, AddressFunds>,
    /// Addresses proven absent from the tree.
    absent: BTreeSet<PlatformP2PKHAddress>,
    /// Known balances from the previous sync, retained for incremental
    /// catch-up's delta-application path.
    known_balances: Vec<(AddressIndex, PlatformAddress, AddressFunds)>,
}

impl PerAccountPlatformAddressState {
    fn new(extended_public_key: ExtendedPubKey) -> Self {
        Self {
            extended_public_key,
            addresses: BiBTreeMap::new(),
            found: BTreeMap::new(),
            absent: BTreeSet::new(),
            known_balances: Vec::new(),
        }
    }
}

/// Per-wallet account map — keys are DIP-17 account indexes (hardened
/// level), values carry the account-level state.
type PerWalletPlatformAddressState = BTreeMap<u32, PerAccountPlatformAddressState>;

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

    /// Wallets currently tracked by this provider.
    #[allow(dead_code)]
    pub(crate) fn wallet_ids(&self) -> impl Iterator<Item = &WalletId> {
        self.per_wallet.keys()
    }

    /// Account indexes covered for a given wallet (sorted, since the
    /// backing `BTreeMap` iterates in key order).
    #[allow(dead_code)]
    pub(crate) fn account_indexes(&self, wallet_id: &WalletId) -> impl Iterator<Item = u32> + '_ {
        self.per_wallet
            .get(wallet_id)
            .into_iter()
            .flat_map(|s| s.keys().copied())
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

    /// Re-populate `pending` for every tracked wallet and account from
    /// their respective `AddressPool`s, and roll the previous pass's
    /// `found` set into `known_balances` for incremental-only delta
    /// application.
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
                // Roll `found` into `known_balances`, recovering the
                // address index via the bimap.
                account_state.known_balances = account_state
                    .found
                    .iter()
                    .filter_map(|(p2pkh, &funds)| {
                        let &index = account_state.addresses.get_by_right(p2pkh)?;
                        Some((index, PlatformAddress::P2pkh(p2pkh.to_bytes()), funds))
                    })
                    .collect();
                account_state.found.clear();
                account_state.absent.clear();
            }
        }
        Ok(())
    }

    /// Update incremental sync state from a completed sync result.
    pub(crate) fn update_sync_state(&mut self, result: &AddressSyncResult<PlatformAddressTag>) {
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

    /// Iterate the most recent pass's found set, scoped to one wallet.
    /// Yields `(account_index, address_index, address, funds)`.
    /// Entries whose address isn't in the account's bimap are skipped —
    /// that would indicate drift between `addresses` and `found`.
    pub(crate) fn found_iter_for_wallet(
        &self,
        wallet_id: &WalletId,
    ) -> impl Iterator<Item = (u32, AddressIndex, &PlatformP2PKHAddress, &AddressFunds)> {
        self.per_wallet
            .get(wallet_id)
            .into_iter()
            .flat_map(|state| {
                state.iter().flat_map(|(&acct, account_state)| {
                    account_state
                        .found
                        .iter()
                        .filter_map(move |(p2pkh, funds)| {
                            let &idx = account_state.addresses.get_by_right(p2pkh)?;
                            Some((acct, idx, p2pkh, funds))
                        })
                })
            })
    }

    /// Iterate the most recent pass's found set across every wallet.
    #[allow(dead_code)]
    pub(crate) fn found_iter(
        &self,
    ) -> impl Iterator<
        Item = (
            WalletId,
            u32,
            AddressIndex,
            &PlatformP2PKHAddress,
            &AddressFunds,
        ),
    > {
        self.per_wallet.iter().flat_map(|(wallet_id, state)| {
            state.iter().flat_map(move |(&acct, account_state)| {
                account_state
                    .found
                    .iter()
                    .filter_map(move |(p2pkh, funds)| {
                        let &idx = account_state.addresses.get_by_right(p2pkh)?;
                        Some((*wallet_id, acct, idx, p2pkh, funds))
                    })
            })
        })
    }

    /// Internal: update the managed account for a newly found address —
    /// record the balance, mark it used in the pool, and extend the
    /// account's pending set to maintain its gap limit.
    async fn on_address_found_in_pool(
        &mut self,
        wallet_id: WalletId,
        account_index: u32,
        p2pkh: &PlatformP2PKHAddress,
        funds: AddressFunds,
    ) -> Result<(), PlatformWalletError> {
        let key_source = self
            .per_wallet
            .get(&wallet_id)
            .and_then(|s| s.get(&account_index))
            .map(|a| KeySource::Public(a.extended_public_key))
            .ok_or_else(|| {
                PlatformWalletError::AddressSync(format!(
                    "No tracked state for wallet {} account {}",
                    hex::encode(wallet_id),
                    account_index
                ))
            })?;

        let mut wm = self.wallet_manager.write().await;
        let info = wm.get_wallet_info_mut(&wallet_id).ok_or_else(|| {
            PlatformWalletError::WalletNotFound(format!(
                "Wallet {:?} not found in wallet manager",
                hex::encode(wallet_id)
            ))
        })?;

        let account = info
            .core_wallet
            .platform_payment_managed_account_at_index_mut(account_index)
            .ok_or_else(|| {
                PlatformWalletError::AddressSync(format!(
                    "No platform payment account at index {}",
                    account_index
                ))
            })?;

        account.set_address_credit_balance(*p2pkh, funds.balance, Some(&key_source));

        // Pick up any newly generated addresses the pool produced when
        // extending to maintain the gap limit.
        let mut new_addresses: Vec<(AddressIndex, PlatformP2PKHAddress)> = Vec::new();
        for (&index, addr_info) in &account.addresses.addresses {
            let key = (wallet_id, account_index, index);
            if self.pending.contains_left(&key) {
                continue;
            }
            let Ok(new_p2pkh) = PlatformP2PKHAddress::from_address(&addr_info.address) else {
                continue;
            };
            new_addresses.push((index, new_p2pkh));
        }
        drop(wm);

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

        Ok(())
    }
}

#[async_trait]
impl AddressProvider for PlatformPaymentAddressProvider {
    /// The engine carries full `(wallet, account, derivation index)`
    /// coordinates through its callbacks, so we don't need to reverse-
    /// lookup from address back to wallet/account ourselves.
    type Tag = PlatformAddressTag;

    /// Fixed at [`DEFAULT_GAP_LIMIT`] (20) until we plumb per-account
    /// gap limits through the unified provider again.
    fn gap_limit(&self) -> AddressIndex {
        DEFAULT_GAP_LIMIT
    }

    fn pending_addresses(
        &self,
    ) -> impl Iterator<Item = (PlatformAddressTag, PlatformAddress)> + '_ {
        self.pending
            .iter()
            .map(|(&tag, p2pkh)| (tag, PlatformAddress::P2pkh(p2pkh.to_bytes())))
    }

    fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    async fn on_address_found(
        &mut self,
        tag: PlatformAddressTag,
        address: &PlatformAddress,
        funds: AddressFunds,
    ) {
        let PlatformAddress::P2pkh(hash) = address else {
            return;
        };
        let p2pkh = PlatformP2PKHAddress::new(*hash);
        let (wallet_id, account_index, address_index) = tag;

        // Consume the pending entry. Missing is fine — the engine can
        // call this on incremental-catch-up hits that were never
        // pending in the first place.
        self.pending.remove_by_left(&tag);

        if let Some(account_state) = self
            .per_wallet
            .get_mut(&wallet_id)
            .and_then(|s| s.get_mut(&account_index))
        {
            account_state.addresses.insert(address_index, p2pkh);
            account_state.found.insert(p2pkh, funds);
        }

        if let Err(e) = self
            .on_address_found_in_pool(wallet_id, account_index, &p2pkh, funds)
            .await
        {
            tracing::warn!(
                "Failed to update pool for found address in wallet {} account {}: {}",
                hex::encode(wallet_id),
                account_index,
                e
            );
        }
    }

    async fn on_address_absent(&mut self, tag: PlatformAddressTag, address: &PlatformAddress) {
        let PlatformAddress::P2pkh(hash) = address else {
            return;
        };
        let p2pkh = PlatformP2PKHAddress::new(*hash);
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
    ) -> impl Iterator<Item = (PlatformAddressTag, PlatformAddress, AddressFunds)> + '_ {
        self.per_wallet
            .iter()
            .flat_map(|(wallet_id, state)| {
                state.iter().map(move |(&account_index, account_state)| {
                    (*wallet_id, account_index, account_state)
                })
            })
            .flat_map(|(wallet_id, account_index, account_state)| {
                account_state
                    .known_balances
                    .iter()
                    .map(move |&(address_index, address, funds)| {
                        ((wallet_id, account_index, address_index), address, funds)
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
