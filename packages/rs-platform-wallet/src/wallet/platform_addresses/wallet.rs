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

        // Reserve the address on hand-out (Found-026): platform-payment
        // `used` only flips on a positive synced balance, so without
        // marking it here a concurrent caller's `next_unused` would
        // re-hand the same index before the sync pass. `mark_index_used`
        // is idempotent — a later real sync hit on this index is a
        // no-op, so gap-limit/`highest_used` accounting isn't doubled.
        let info = managed_account
            .addresses
            .next_unused_with_info(&key_source, true)
            .map_err(|e| PlatformWalletError::AddressSync(e.to_string()))?;
        let address = info.address.clone();
        managed_account.addresses.mark_index_used(info.index);

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
            balance: Arc::new(crate::wallet::core::WalletBalance::new()),
            identity_manager: crate::wallet::identity::IdentityManager::new(),
            tracked_asset_locks: BTreeMap::new(),
        };
        let wallet_manager = Arc::new(RwLock::new(WalletManager::new(Network::Testnet)));
        let wallet_id = wallet_manager
            .try_write()
            .expect("uncontended")
            .insert_wallet(wallet, info)
            .expect("insert");
        let persister = WalletPersister::new(wallet_id, Arc::new(NoPlatformPersistence));
        PlatformAddressWallet::new(sdk, wallet_manager, wallet_id, persister)
    }

    /// Found-026 durable guard: two `next_unused_receive_address` calls
    /// with NO intervening sync/balance update must return DISTINCT
    /// addresses. Pre-fix, `next_unused` re-hands index 0 (its `used`
    /// flag only flips on a positive synced balance) → identical
    /// addresses → this assertion fails. Post-fix the first call
    /// reserves index 0 via `mark_index_used`, so the second yields
    /// index 1.
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

    /// Found-026: K repeated hand-outs advance `highest_used` /
    /// `used_indices` by exactly K (no double-count, no skipped index,
    /// no panic), all addresses distinct; and a subsequent
    /// `mark_index_used` on an already-reserved index is a no-op
    /// (idempotency — the later real sync hit must not double-count).
    #[tokio::test]
    async fn found_026_repeated_handouts_advance_gap_limit_exactly_k() {
        const K: u32 = 5;
        let wallet = wallet_with_platform_account();

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

        assert_eq!(
            pool.highest_used,
            Some(K - 1),
            "highest_used must advance to exactly K-1 (no double-count / skip)"
        );
        assert_eq!(
            pool.used_indices.len(),
            K as usize,
            "exactly K indices reserved"
        );

        // Idempotency: re-marking an already-reserved index (the shape
        // of a later real sync hit on a handed-out address) is a no-op.
        assert!(
            !pool.mark_index_used(0),
            "re-marking a reserved index must be a no-op (idempotent)"
        );
        assert_eq!(
            pool.highest_used,
            Some(K - 1),
            "no-op re-mark must not perturb highest_used"
        );
        assert_eq!(
            pool.used_indices.len(),
            K as usize,
            "no-op re-mark must not perturb used_indices"
        );
    }
}
