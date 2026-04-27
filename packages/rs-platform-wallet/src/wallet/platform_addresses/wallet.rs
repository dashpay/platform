//! Platform address wallet for DIP-17 platform payment addresses.

use std::sync::Arc;

use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use key_wallet::PlatformP2PKHAddress;
use tokio::sync::RwLock;

use crate::error::PlatformWalletError;
use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
use key_wallet_manager::WalletManager;

use crate::wallet::persister::WalletPersister;

use super::provider::PlatformPaymentAddressProvider;

/// DIP-17 derivation coordinates for an address owned by a
/// [`PlatformAddressWallet`].
///
/// Surfaced by [`PlatformAddressWallet::address_derivation_info`] so
/// external [`Signer<PlatformAddress>`](dpp::identity::signer::Signer)
/// implementations can re-derive the matching ECDSA private key from
/// the wallet seed at the DIP-17 path:
///
/// `m/9'/coin_type'/17'/account_index'/key_class'/key_index`
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AddressDerivationInfo {
    /// DIP-17 account index (hardened level).
    pub account_index: u32,
    /// DIP-17 key-class index (hardened level) — selects key purpose.
    /// `0` denotes the clear-funds payment key class. Mirrors
    /// `key_wallet`'s
    /// [`PlatformPaymentAccountKey::key_class`](key_wallet::account::account_collection::PlatformPaymentAccountKey).
    pub key_class: u32,
    /// Address derivation index within the
    /// `(account_index, key_class)` subtree.
    pub key_index: u32,
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
    /// wallets don't allocate empty state. Sync takes a `write` lock;
    /// transfer/withdraw paths take `read` for key_source lookups.
    pub(crate) provider: Arc<RwLock<Option<PlatformPaymentAddressProvider>>>,
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
        persister: WalletPersister,
    ) -> Self {
        Self {
            sdk,
            wallet_manager,
            wallet_id,
            provider: Arc::new(RwLock::new(None)),
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
    pub async fn initialize_from_persisted(
        &self,
        persisted: crate::PlatformAddressSyncStartState,
    ) -> Result<(), PlatformWalletError> {
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

    /// Get the network from the SDK.
    pub fn network(&self) -> key_wallet::Network {
        self.sdk.network
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
            .next_unused(&key_source)
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

    /// Look up the DIP-17 derivation info for an address owned by this
    /// wallet.
    ///
    /// Returns `Some(AddressDerivationInfo { account_index, key_class,
    /// key_index })` when `addr` belongs to one of this wallet's
    /// tracked platform-payment accounts; `None` otherwise. `None` is
    /// also returned for:
    ///
    /// - P2SH addresses (platform-payment accounts derive only P2PKH).
    /// - Addresses for an account that has not been initialized via
    ///   [`Self::initialize`] yet.
    /// - Addresses derived under a `(account, key_class)` pair whose
    ///   xpub does not appear in the wallet's
    ///   `platform_payment_accounts` map (i.e. account drift between
    ///   the provider and the wallet manager — should not happen in
    ///   normal operation).
    ///
    /// Useful for external
    /// [`Signer<PlatformAddress>`](dpp::identity::signer::Signer)
    /// implementations that need to re-derive the matching ECDSA
    /// private key from the seed without poking at the wallet manager
    /// directly.
    pub async fn address_derivation_info(
        &self,
        addr: &PlatformAddress,
    ) -> Option<AddressDerivationInfo> {
        // Platform-payment accounts only derive P2PKH; bail out fast
        // on any other variant rather than searching the provider.
        let p2pkh = match addr {
            PlatformAddress::P2pkh(bytes) => PlatformP2PKHAddress::new(*bytes),
            PlatformAddress::P2sh(_) => return None,
        };

        // Phase 1: provider holds the (account_index, key_index, xpub)
        // bijection for every tracked address — but key_class isn't
        // stored alongside, so we capture the xpub here and recover
        // key_class against the wallet's account map below.
        let (account_index, key_index, xpub) = {
            let provider_guard = self.provider.read().await;
            provider_guard
                .as_ref()?
                .lookup_p2pkh(&self.wallet_id, &p2pkh)?
        };

        // Phase 2: walk the wallet's platform_payment_accounts map and
        // pick the entry whose `(account, account_xpub)` matches the
        // tuple captured above. Multiple key classes per account index
        // are possible in principle (DIP-17), so xpub equality is the
        // disambiguator.
        let wm = self.wallet_manager.read().await;
        let wallet = wm.get_wallet(&self.wallet_id)?;
        let key_class =
            wallet
                .accounts
                .platform_payment_accounts
                .iter()
                .find_map(|(key, acct)| {
                    if key.account == account_index && acct.account_xpub == xpub {
                        Some(key.key_class)
                    } else {
                        None
                    }
                })?;

        Some(AddressDerivationInfo {
            account_index,
            key_class,
            key_index,
        })
    }
}

impl std::fmt::Debug for PlatformAddressWallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlatformAddressWallet")
            .field("network", &self.sdk.network)
            .finish()
    }
}
