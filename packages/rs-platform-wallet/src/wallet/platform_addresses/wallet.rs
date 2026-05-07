//! Platform address wallet for DIP-17 platform payment addresses.

use std::sync::Arc;

use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use tokio::sync::RwLock;

use crate::error::PlatformWalletError;
use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
use key_wallet_manager::WalletManager;

use crate::wallet::persister::WalletPersister;

use super::provider::PlatformPaymentAddressProvider;

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

    /// Derive `count` consecutive UNUSED receive addresses, always
    /// extending past `highest_generated`.
    ///
    /// Unlike [`Self::next_unused_receive_address`] (which parks on the
    /// LOWEST unused index until something marks it used), this accessor
    /// permanently advances the address pool's `highest_generated`
    /// watermark on every call, so consecutive invocations on the same
    /// wallet yield non-overlapping ranges. This is the contract PA-005b
    /// pins at the `gap_limit` boundary.
    ///
    /// **Gap-limit interaction**: an `AddressPool` exposes `gap_limit`
    /// unused addresses past the highest-used index (or `gap_limit`
    /// total when nothing is used yet). If `count` would push the unused
    /// run past that ceiling — i.e. `(highest_generated + count) -
    /// highest_used > gap_limit` — the call returns
    /// [`PlatformWalletError::GapLimitExceeded`] without mutating pool
    /// state. Callers can mark an address used (e.g. by funding it) to
    /// open more headroom and retry.
    pub async fn next_unused_receive_addresses(
        &self,
        account_key: key_wallet::account::account_collection::PlatformPaymentAccountKey,
        count: usize,
    ) -> Result<Vec<PlatformAddress>, PlatformWalletError> {
        if count == 0 {
            return Ok(Vec::new());
        }

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

        let addresses =
            derive_fresh_unused_addresses(&mut managed_account.addresses, &key_source, count)?;

        addresses
            .into_iter()
            .map(|address| {
                PlatformAddress::try_from(address).map_err(|e| {
                    PlatformWalletError::AddressSync(format!(
                        "Failed to convert to PlatformAddress: {e}"
                    ))
                })
            })
            .collect()
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

    /// Read the current incremental-sync watermark from the unified
    /// platform-address provider.
    ///
    /// Returns `None` when the provider hasn't been initialised yet
    /// (no [`Self::initialize`] call) or when the provider has no stored
    /// watermark (whether restored via [`Self::apply_sync_state`] or
    /// produced by a previous sync). The value is monotonic non-decreasing
    /// across [`Self::sync_balances`](super::sync) calls against the
    /// same chain — a later sync can only advance the watermark, never
    /// roll it back. A zero-valued watermark is reported as `None` to
    /// match the "no stored watermark" convention used elsewhere in
    /// the wallet (see [`Self::apply_sync_state`]).
    pub async fn sync_watermark(&self) -> Option<u64> {
        let guard = self.provider.read().await;
        let raw = guard.as_ref().map(|p| p.last_known_recent_block())?;
        if raw == 0 {
            None
        } else {
            Some(raw)
        }
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

/// Allocate `count` fresh, unused addresses past the pool's
/// `highest_generated` watermark.
///
/// Unlike [`AddressPool::next_unused_multiple`] this never recycles
/// already-issued unused indices — every returned address is a freshly
/// derived index. The operation is gated by the pool's gap-limit:
/// requesting more than the current headroom returns
/// [`PlatformWalletError::GapLimitExceeded`] without mutating pool
/// state. Caller is expected to hold an exclusive (`&mut`) borrow of
/// the pool.
fn derive_fresh_unused_addresses(
    pool: &mut key_wallet::AddressPool,
    key_source: &key_wallet::KeySource,
    count: usize,
) -> Result<Vec<key_wallet::Address>, PlatformWalletError> {
    if count == 0 {
        return Ok(Vec::new());
    }

    // Headroom = (highest_used + gap_limit) - highest_generated, where
    // missing watermarks fall back to the empty-pool case (highest_used
    // absent ⇒ ceiling at gap_limit-1; highest_generated absent ⇒
    // start at index 0). All arithmetic stays in u32: gap_limit is u32
    // and the watermarks are u32.
    let gap_limit = pool.gap_limit;
    let ceiling: u32 = match pool.highest_used {
        None => gap_limit.saturating_sub(1),
        Some(highest) => highest.saturating_add(gap_limit),
    };
    let next_index: u32 = pool
        .highest_generated
        .map(|h| h.saturating_add(1))
        .unwrap_or(0);
    let available: u32 = ceiling.saturating_sub(next_index).saturating_add(1);
    let count_u32 = u32::try_from(count).unwrap_or(u32::MAX);
    if count_u32 > available {
        return Err(PlatformWalletError::GapLimitExceeded {
            requested: count,
            available,
            highest_used: pool.highest_used,
            highest_generated: pool.highest_generated,
            gap_limit,
        });
    }

    pool.generate_addresses(count_u32, key_source, true)
        .map_err(|e| PlatformWalletError::AddressSync(e.to_string()))
}

#[cfg(test)]
mod next_unused_receive_addresses_tests {
    //! Unit tests for the pool-level helper backing
    //! [`PlatformAddressWallet::next_unused_receive_addresses`].
    //! Driving the wallet entry point directly requires a full
    //! `WalletManager + Sdk` fixture, which is heavyweight and
    //! exercised in e2e (PA-005b). The helper itself is the meaningful
    //! contract — the wallet method is a thin lock-and-lookup wrapper.
    use super::derive_fresh_unused_addresses;
    use crate::error::PlatformWalletError;
    use key_wallet::bip32::{ChildNumber, DerivationPath, ExtendedPrivKey};
    use key_wallet::dashcore::secp256k1::Secp256k1;
    use key_wallet::managed_account::address_pool::{AddressPool, AddressPoolType};
    use key_wallet::mnemonic::{Language, Mnemonic};
    use key_wallet::{KeySource, Network};

    fn test_key_source() -> KeySource {
        let mnemonic = Mnemonic::from_phrase(
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
            Language::English,
        )
        .expect("mnemonic parses");
        let seed = mnemonic.to_seed("");
        let master = ExtendedPrivKey::new_master(Network::Testnet, &seed).expect("master xprv");
        let secp = Secp256k1::new();
        let path = DerivationPath::from(vec![
            ChildNumber::from_hardened_idx(44).unwrap(),
            ChildNumber::from_hardened_idx(1).unwrap(),
            ChildNumber::from_hardened_idx(0).unwrap(),
        ]);
        let account_key = master
            .derive_priv(&secp, &path)
            .expect("account derivation");
        KeySource::Private(account_key)
    }

    fn empty_pool(gap_limit: u32) -> AddressPool {
        let base_path = DerivationPath::from(vec![ChildNumber::from_normal_idx(0).unwrap()]);
        AddressPool::new_without_generation(
            base_path,
            AddressPoolType::External,
            gap_limit,
            Network::Testnet,
        )
    }

    #[test]
    fn returns_count_addresses_all_distinct() {
        let mut pool = empty_pool(20);
        let key_source = test_key_source();
        let addrs = derive_fresh_unused_addresses(&mut pool, &key_source, 19)
            .expect("19 ≤ gap_limit, must succeed");
        assert_eq!(addrs.len(), 19);
        let unique: std::collections::HashSet<_> = addrs.iter().collect();
        assert_eq!(unique.len(), 19, "all 19 addresses must be distinct");
        assert_eq!(pool.highest_generated, Some(18));
    }

    #[test]
    fn consecutive_calls_yield_non_overlapping_ranges() {
        let mut pool = empty_pool(20);
        let key_source = test_key_source();
        let first = derive_fresh_unused_addresses(&mut pool, &key_source, 5)
            .expect("first batch fits in gap_limit");
        // After 5 generated and none used, headroom is 20 - 5 = 15;
        // request another 5 to lock the non-overlap contract.
        let second = derive_fresh_unused_addresses(&mut pool, &key_source, 5)
            .expect("second batch fits in remaining headroom");
        assert_eq!(first.len(), 5);
        assert_eq!(second.len(), 5);
        let intersection: std::collections::HashSet<_> = first.iter().collect();
        assert!(
            second.iter().all(|a| !intersection.contains(a)),
            "consecutive calls must not return any overlapping address"
        );
        assert_eq!(pool.highest_generated, Some(9));
    }

    #[test]
    fn does_not_exceed_gap_limit_cap() {
        let gap_limit = 20;
        let mut pool = empty_pool(gap_limit);
        let key_source = test_key_source();
        // No used indices ⇒ ceiling at index gap_limit-1=19, headroom = gap_limit = 20.
        // Requesting 21 must error rather than over-extend.
        let err = derive_fresh_unused_addresses(&mut pool, &key_source, 21).unwrap_err();
        match err {
            PlatformWalletError::GapLimitExceeded {
                requested,
                available,
                gap_limit: gl,
                ..
            } => {
                assert_eq!(requested, 21);
                assert_eq!(available, 20);
                assert_eq!(gl, gap_limit);
            }
            other => panic!("expected GapLimitExceeded, got {:?}", other),
        }
        // Pool must remain untouched after a rejected request.
        assert_eq!(pool.highest_generated, None);
    }

    #[test]
    fn count_zero_is_no_op() {
        let mut pool = empty_pool(20);
        let key_source = test_key_source();
        let addrs = derive_fresh_unused_addresses(&mut pool, &key_source, 0)
            .expect("count = 0 is a no-op success");
        assert!(addrs.is_empty());
        assert_eq!(pool.highest_generated, None);
    }

    #[test]
    fn marking_used_extends_headroom() {
        // Once an index is marked used, the gap-limit ceiling shifts
        // up by `gap_limit`, so a subsequent request that would have
        // exceeded the original cap can succeed.
        let gap_limit = 20;
        let mut pool = empty_pool(gap_limit);
        let key_source = test_key_source();
        let first = derive_fresh_unused_addresses(&mut pool, &key_source, gap_limit as usize)
            .expect("first batch fits exactly in initial gap_limit window");
        assert_eq!(first.len(), gap_limit as usize);
        // Mark the lowest one used to advance highest_used to 0; new
        // ceiling = 0 + gap_limit = 20, but highest_generated is 19,
        // so headroom = 1 fresh address.
        pool.mark_used(&first[0]);
        let second =
            derive_fresh_unused_addresses(&mut pool, &key_source, 1).expect("one more fits");
        assert_eq!(second.len(), 1);
        assert!(!first.contains(&second[0]));
    }
}
