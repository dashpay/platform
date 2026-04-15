//! DIP-17 platform payment address provider for HD wallet scanning.
//!
//! Delegates address derivation and gap-limit management to key-wallet's
//! [`ManagedPlatformAccount`] / [`AddressPool`] instead of reimplementing
//! HD logic locally.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use dpp::address_funds::PlatformAddress;
use key_wallet::account::account_collection::PlatformPaymentAccountKey;
use key_wallet::managed_account::address_pool::KeySource;
use key_wallet::PlatformP2PKHAddress;

use key_wallet_manager::WalletManager;
use tokio::sync::RwLock;

use crate::error::PlatformWalletError;
use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
use dash_sdk::platform::address_sync::{
    AddressFunds, AddressIndex, AddressProvider, AddressSyncResult,
};

/// Internal address provider implementing [`AddressProvider`] for DIP-17
/// platform payment address discovery.
///
/// Reads pre-generated addresses from key-wallet's [`AddressPool`] and
/// delegates gap-limit extension back to the pool when new addresses are
/// found during sync.
pub(crate) struct PlatformPaymentAddressAccountProvider {
    /// Shared wallet manager for gap-limit extension.
    wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    /// Identifies which wallet within the manager this provider operates on.
    wallet_id: WalletId,
    /// Platform payment account index (DIP-17 account hardened level).
    account_index: u32,
    /// Gap limit cached from the AddressPool at construction time.
    cached_gap_limit: u32,
    /// Cached public key source for address generation (no private key needed).
    key_source: KeySource,
    /// Pending addresses from the pool: index -> platform address.
    pending: BTreeMap<AddressIndex, PlatformP2PKHAddress>,
    /// Addresses found with their balances.
    found: BTreeMap<(AddressIndex, PlatformP2PKHAddress), AddressFunds>,
    /// Addresses proven absent from the tree.
    absent: BTreeSet<(AddressIndex, PlatformP2PKHAddress)>,
    /// Highest index found with a non-zero balance.
    highest_found: Option<AddressIndex>,
    /// Previously known balances from the last sync (for incremental-only mode).
    known_balances: Vec<(AddressIndex, PlatformAddress, AddressFunds)>,
    /// Last sync height from the previous sync (for incremental catch-up resume).
    sync_height: u64,
    /// Last sync timestamp from the previous sync (for full-rescan-after threshold).
    sync_timestamp: u64,
    /// Last known recent block height from the previous sync (for compaction detection).
    last_known_recent_block: u64,
}

impl PlatformPaymentAddressAccountProvider {
    /// Create an address provider from a wallet.
    ///
    /// Reads the initial set of pre-generated addresses from the platform
    /// payment account at `account_index`. No key derivation happens here
    /// — addresses were already generated when the account was created.
    pub(crate) fn from_wallet(
        wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        wallet_id: WalletId,
        account_index: u32,
    ) -> Result<Self, PlatformWalletError> {
        let (cached_gap_limit, key_source, pending) = {
            let wm = wallet_manager.blocking_read();
            let (wallet, info) = wm.get_wallet_and_info(&wallet_id).ok_or_else(|| {
                PlatformWalletError::WalletNotFound(format!(
                    "Wallet {:?} not found in wallet manager",
                    hex::encode(wallet_id)
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

            let cached_gap_limit = account.gap_limit();

            // Cache the account's public key source for address generation.
            let account_key = PlatformPaymentAccountKey {
                account: account_index,
                key_class: 0,
            };
            let xpub = wallet
                .accounts
                .platform_payment_accounts
                .get(&account_key)
                .map(|a| a.account_xpub)
                .ok_or_else(|| {
                    PlatformWalletError::AddressSync(format!(
                        "No platform payment account key at index {}",
                        account_index
                    ))
                })?;
            let key_source = KeySource::Public(xpub);

            // Read all pre-generated addresses from the pool.
            let mut pending = BTreeMap::new();
            for (&index, addr_info) in &account.addresses.addresses {
                if let Ok(p2pkh) = PlatformP2PKHAddress::from_address(&addr_info.address) {
                    pending.insert(index, p2pkh);
                }
            }

            (cached_gap_limit, key_source, pending)
        };

        Ok(Self {
            wallet_manager,
            wallet_id,
            account_index,
            cached_gap_limit,
            key_source,
            pending,
            found: BTreeMap::new(),
            absent: BTreeSet::new(),
            highest_found: None,
            known_balances: Vec::new(),
            sync_height: 0,
            sync_timestamp: 0,
            last_known_recent_block: 0,
        })
    }

    /// The cached public key source for address generation.
    pub(crate) fn key_source(&self) -> &KeySource {
        &self.key_source
    }

    /// The last sync timestamp, or `None` if never synced.
    pub(crate) fn last_sync_timestamp(&self) -> Option<u64> {
        if self.sync_timestamp == 0 {
            None
        } else {
            Some(self.sync_timestamp)
        }
    }

    /// Re-populate `pending` from the address pool and update `known_balances`
    /// from the previous sync's `found` results. Call this before each sync
    /// to prepare the provider for a new round.
    pub(crate) fn prepare_for_sync(&mut self) -> Result<(), PlatformWalletError> {
        let wm = self.wallet_manager.blocking_read();
        let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
            PlatformWalletError::WalletNotFound(format!(
                "Wallet {:?} not found in wallet manager",
                hex::encode(self.wallet_id)
            ))
        })?;

        let account = info
            .core_wallet
            .platform_payment_managed_account_at_index(self.account_index)
            .ok_or_else(|| {
                PlatformWalletError::AddressSync(format!(
                    "No platform payment account at index {}",
                    self.account_index
                ))
            })?;

        // Refresh pending from the pool.
        self.pending.clear();
        for (&index, addr_info) in &account.addresses.addresses {
            if let Ok(p2pkh) = PlatformP2PKHAddress::from_address(&addr_info.address) {
                self.pending.insert(index, p2pkh);
            }
        }

        // Carry forward found balances as known_balances for incremental mode.
        self.known_balances = self
            .found
            .iter()
            .map(|(&(index, p2pkh), &funds)| {
                (index, PlatformAddress::P2pkh(p2pkh.to_bytes()), funds)
            })
            .collect();
        self.found.clear();
        self.absent.clear();

        Ok(())
    }

    /// Update incremental sync state from a completed sync result.
    pub(crate) fn update_sync_state(&mut self, result: &AddressSyncResult) {
        self.sync_height = result.new_sync_height;
        self.sync_timestamp = result.new_sync_timestamp;
        self.last_known_recent_block = result.last_known_recent_block;
    }

    /// Update the account for a found address: set its balance, mark it used
    /// in the pool, and generate new addresses to maintain the gap limit.
    ///
    /// Asynchronous because it acquires the wallet manager's async write lock,
    /// so it must never be called from a `blocking_*` context on a tokio
    /// worker thread.
    async fn on_address_found_in_pool(
        &mut self,
        p2pkh: &PlatformP2PKHAddress,
        funds: AddressFunds,
    ) -> Result<(), PlatformWalletError> {
        let mut wm = self.wallet_manager.write().await;
        let info = wm.get_wallet_info_mut(&self.wallet_id).ok_or_else(|| {
            PlatformWalletError::WalletNotFound(format!(
                "Wallet {:?} not found in wallet manager",
                hex::encode(self.wallet_id)
            ))
        })?;

        let account = info
            .core_wallet
            .platform_payment_managed_account_at_index_mut(self.account_index)
            .ok_or_else(|| {
                PlatformWalletError::AddressSync(format!(
                    "No platform payment account at index {}",
                    self.account_index
                ))
            })?;

        // Set balance, mark used, and maintain gap limit — all in one call.
        account.set_address_credit_balance(*p2pkh, funds.balance, Some(&self.key_source));

        // Add any newly generated addresses to pending.
        for (&index, addr_info) in &account.addresses.addresses {
            if !self.pending.contains_key(&index) {
                if let Ok(new_p2pkh) = PlatformP2PKHAddress::from_address(&addr_info.address) {
                    self.pending.insert(index, new_p2pkh);
                }
            }
        }

        Ok(())
    }
}

impl AddressProvider for PlatformPaymentAddressAccountProvider {
    fn gap_limit(&self) -> AddressIndex {
        self.cached_gap_limit
    }

    fn pending_addresses(&self) -> Vec<(AddressIndex, PlatformAddress)> {
        self.pending
            .iter()
            .map(|(index, p2pkh)| (*index, PlatformAddress::P2pkh(p2pkh.to_bytes())))
            .collect()
    }

    async fn on_address_found(
        &mut self,
        index: AddressIndex,
        address: &PlatformAddress,
        funds: AddressFunds,
    ) {
        let PlatformAddress::P2pkh(hash) = address else {
            return;
        };
        let p2pkh = PlatformP2PKHAddress::new(*hash);

        self.pending.remove(&index);
        self.found.insert((index, p2pkh), funds);
        self.highest_found = Some(self.highest_found.map_or(index, |v| v.max(index)));

        if let Err(e) = self.on_address_found_in_pool(&p2pkh, funds).await {
            tracing::warn!("Failed to update pool for found address: {}", e);
        }
    }

    async fn on_address_absent(&mut self, index: AddressIndex, address: &PlatformAddress) {
        let PlatformAddress::P2pkh(hash) = address else {
            return;
        };
        self.pending.remove(&index);
        self.absent
            .insert((index, PlatformP2PKHAddress::new(*hash)));
    }

    fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    fn highest_found_index(&self) -> Option<AddressIndex> {
        self.highest_found
    }

    fn current_balances(&self) -> &[(AddressIndex, PlatformAddress, AddressFunds)] {
        &self.known_balances
    }

    fn last_sync_height(&self) -> u64 {
        self.sync_height
    }

    fn last_known_recent_block_height(&self) -> u64 {
        self.last_known_recent_block
    }
}
