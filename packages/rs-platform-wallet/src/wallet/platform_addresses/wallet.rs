//! Platform address wallet for DIP-17 platform payment addresses.

use std::sync::Arc;

use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use tokio::sync::RwLock;

use crate::broadcaster::SpvBroadcaster;
use crate::error::PlatformWalletError;
use crate::wallet::asset_lock::manager::AssetLockManager;
use crate::wallet::core::reservations::AddressReservations;
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
    /// Shared asset-lock manager. Threaded in so the orchestrated
    /// `fund_from_asset_lock` path can drive
    /// build → IS-or-CL wait → consume on the same tracked locks
    /// every other sub-wallet sees. Cloned `Arc`, not owned.
    pub(crate) asset_locks: Arc<AssetLockManager<SpvBroadcaster>>,
    /// Per-wallet persistence handle for queuing changesets.
    pub(crate) persister: WalletPersister,
    /// `(account_index, address_index)` slots handed out by
    /// [`next_unused_receive_address`](Self::next_unused_receive_address)
    /// but not yet flipped to `used` by a positive-balance sync hit.
    /// Closes the Found-026 back-to-back hand-out race without
    /// eagerly advancing the upstream `AddressPool`'s `highest_used` —
    /// the upstream flip happens when a real payment lands on the
    /// address.
    pub(crate) reservations: AddressReservations,
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
            reservations: AddressReservations::new(),
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

        // Found-026 / #3658-review: reserve the slot in an in-memory
        // `AddressReservations` set rather than flipping the upstream
        // pool's `used` flag. Hand-out must not advance `highest_used`
        // (gap-limit accounting) until the address actually receives a
        // payment — chain sync flips `used` on the positive-balance
        // hit, at which point the reservation becomes redundant.
        //
        // The reservation must filter the pool's `next_unused_with_info`
        // result: that function only knows about the pool's own `used`
        // flag, so we scan forward from index 0 picking the first slot
        // that is neither `used` nor reserved. If every generated index
        // is taken, mint a new one via `address_range`.
        let reserved = self.reservations.snapshot();
        let account_index = account_key.account;
        let pool = &mut managed_account.addresses;

        let pick = (0..=pool.highest_generated.unwrap_or(0)).find_map(|i| {
            pool.addresses.get(&i).and_then(|info| {
                if info.used || reserved.contains(&(account_index, i)) {
                    None
                } else {
                    Some(info.clone())
                }
            })
        });

        let address_info = match pick {
            Some(info) => info,
            None => {
                // No unused-and-unreserved index exists in the
                // already-generated range: mint a fresh one strictly
                // beyond `highest_generated`. Two cases:
                //
                // 1. `highest_generated` is `Some(h)` — `address_range(h+1, h+2)`
                //    generates exactly one new address at index `h+1`.
                // 2. `highest_generated` is `None` (pool empty) —
                //    `address_range`'s `unwrap_or(0)` for the "current
                //    highest" collides with the "no indices exist"
                //    state, so a `(0, 1)` range yields an empty vec.
                //    Fall back to the upstream `next_unused_with_info`,
                //    which is correct in the empty case (no `!used &&
                //    !reserved` index can exist if no indices exist).
                match pool.highest_generated {
                    Some(h) => {
                        let next_index = h + 1;
                        pool.address_range(next_index, next_index + 1, &key_source)
                            .map_err(|e| PlatformWalletError::AddressSync(e.to_string()))?;
                        pool.info_at_index(next_index).cloned().ok_or_else(|| {
                            PlatformWalletError::AddressSync(format!(
                                "Failed to retrieve generated address info at index {next_index}"
                            ))
                        })?
                    }
                    None => pool
                        .next_unused_with_info(&key_source, true)
                        .map_err(|e| PlatformWalletError::AddressSync(e.to_string()))?,
                }
            }
        };

        // INTENTIONAL(CMT-001 / #3658 review): the reservation is
        // in-memory only by design. A crash before the next sync AND
        // no payment received by restart manifests as address-reuse
        // (a privacy/accounting nuisance, not fund loss). Addresses
        // requested but never paid to are freed for reuse on the next
        // session, avoiding unbounded index-gap growth from
        // speculative or abandoned hand-outs. The eager
        // `mark_index_used` flip was removed at QuantumExplorer's
        // request because it advanced `highest_used` permanently —
        // see #3658 review thread.
        self.reservations
            .reserve(vec![(account_index, address_info.index)])
            .leak_until_sync();
        let address = address_info.address;

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

        // Asset-lock manager is required by the constructor but never
        // exercised here — these tests only drive receive-address hand-out.
        let event_manager = Arc::new(crate::events::PlatformEventManager::new(Vec::new()));
        let spv = Arc::new(crate::spv::SpvRuntime::new(
            Arc::clone(&wallet_manager),
            event_manager,
        ));
        let broadcaster = Arc::new(SpvBroadcaster::new(spv));
        let asset_locks = Arc::new(AssetLockManager::new(
            Arc::clone(&sdk),
            Arc::clone(&wallet_manager),
            wallet_id,
            Arc::new(tokio::sync::Notify::new()),
            broadcaster,
            persister.clone(),
        ));
        PlatformAddressWallet::new(sdk, wallet_manager, wallet_id, asset_locks, persister)
    }

    /// Found-026 durable guard: two `next_unused_receive_address` calls
    /// with NO intervening sync/balance update must return DISTINCT
    /// addresses. Pre-fix, `next_unused` re-hands index 0 (its `used`
    /// flag only flips on a positive synced balance) → identical
    /// addresses → this assertion fails. Post-fix the first call
    /// reserves index 0 in the in-memory `AddressReservations` set,
    /// so the second call skips it and yields index 1. The upstream
    /// pool's `highest_used` is left untouched until a real payment
    /// lands — see #3658 review thread.
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

    /// `next_unused_receive_address` must not advance `highest_used`
    /// in the upstream `AddressPool` — that was @QuantumExplorer's
    /// review concern on #3658. The reservation should be in-memory
    /// only (gap-limit accounting stays anchored to actually-paid
    /// addresses).
    #[tokio::test]
    async fn next_unused_receive_address_does_not_advance_highest_used() {
        let wallet = wallet_with_platform_account();

        let _a = wallet
            .next_unused_receive_address(ACCOUNT_KEY)
            .await
            .expect("first hand-out");
        let _b = wallet
            .next_unused_receive_address(ACCOUNT_KEY)
            .await
            .expect("second hand-out");

        let wm = wallet.wallet_manager.read().await;
        let info = wm.get_wallet_info(&wallet.wallet_id).expect("wallet info");
        let account = info
            .core_wallet
            .platform_payment_managed_account_at_index(ACCOUNT_KEY.account)
            .expect("account");
        assert_eq!(
            account.addresses.highest_used, None,
            "hand-out must not advance `highest_used` — reservations are in-memory only"
        );
    }

    /// Three consecutive hand-outs (no sync, no payment) must yield
    /// three distinct indices. Pre-fix with `mark_index_used` removed,
    /// a naive "scan for first `!used`" would re-hand index 0 because
    /// `used` never flips. The `AddressReservations` filter must skip
    /// reserved-but-not-used slots to maintain the distinct-handout
    /// invariant across more than two calls.
    #[tokio::test]
    async fn handout_skips_reserved_but_not_used_indices() {
        let wallet = wallet_with_platform_account();

        let a = wallet
            .next_unused_receive_address(ACCOUNT_KEY)
            .await
            .expect("first hand-out");
        let b = wallet
            .next_unused_receive_address(ACCOUNT_KEY)
            .await
            .expect("second hand-out");
        let c = wallet
            .next_unused_receive_address(ACCOUNT_KEY)
            .await
            .expect("third hand-out");

        assert_ne!(a, b, "1st vs 2nd hand-out collided");
        assert_ne!(a, c, "1st vs 3rd hand-out collided");
        assert_ne!(b, c, "2nd vs 3rd hand-out collided");
    }
}
