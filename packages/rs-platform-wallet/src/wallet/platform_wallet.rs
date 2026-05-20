//! The main PlatformWallet struct combining core, identity (+DashPay), and platform sub-wallets.

use std::collections::BTreeMap;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use dashcore::OutPoint;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use key_wallet_manager::WalletManager;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use super::asset_lock::manager::AssetLockManager;
use super::asset_lock::tracked::TrackedAssetLock;
use super::core::{CoreWallet, WalletBalance};
use super::identity::{IdentityManager, IdentityWallet};
use super::persister::WalletPersister;
use super::platform_addresses::PlatformAddressWallet;
#[cfg(feature = "shielded")]
use super::shielded::{FileBackedShieldedStore, ShieldedWallet};
use crate::broadcaster::SpvBroadcaster;
use crate::changeset::{
    ClientStartState, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
};
#[cfg(feature = "shielded")]
use crate::error::PlatformWalletError;

/// Unique identifier for a wallet (32-byte hash).
pub type WalletId = [u8; 32];

/// Consolidated mutable state for a platform wallet.
///
/// Lives inside `WalletManager<PlatformWalletInfo>.wallet_infos`. The `Wallet`
/// key material is in `WalletManager.wallets` — NOT inside this struct.
///
/// `WalletBalance` is stored as `Arc<WalletBalance>` for lock-free UI reads.
pub struct PlatformWalletInfo {
    /// Core wallet metadata, accounts, UTXOs, balances.
    /// Delegates `WalletInfoInterface` methods.
    pub core_wallet: ManagedWalletInfo,
    /// Lock-free balance for UI reads. Updated from `ManagedWalletInfo` after
    /// each SPV block/mempool processing and RPC refresh.
    pub balance: Arc<WalletBalance>,
    pub identity_manager: IdentityManager,
    pub tracked_asset_locks: BTreeMap<OutPoint, TrackedAssetLock>,
}

/// A platform wallet that combines core UTXO functionality with identity management.
///
/// This is SPV-free. It needs only key material and an `Sdk`.
/// For SPV support, use [`PlatformWalletManager`](crate::manager::PlatformWalletManager).
///
/// # Cloning
///
/// `PlatformWallet` is cheaply cloneable (a few atomic increments). A clone is a
/// **shared handle** to the same mutable state — not an independent copy. All
/// clones see the same UTXOs, balances, and identities through the shared
/// `WalletManager` lock.
pub struct PlatformWallet {
    wallet_id: WalletId,
    pub(crate) sdk: Arc<dash_sdk::Sdk>,
    pub(crate) wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    // Sub-wallets that hold a broadcaster are monomorphized with
    // `SpvBroadcaster` — the only production broadcaster in use.
    // Swapping this out to another broadcaster is a three-line flip
    // right here plus the `new()` signature below; the sub-wallet
    // definitions themselves stay untouched.
    pub(crate) core: CoreWallet<SpvBroadcaster>,
    pub(crate) identity: IdentityWallet<SpvBroadcaster>,
    pub(crate) platform: PlatformAddressWallet,
    /// Shared asset lock manager.
    pub(crate) asset_locks: Arc<AssetLockManager<SpvBroadcaster>>,
    /// Per-wallet persistence handle.
    persister: WalletPersister,
    /// Lock-free balance for UI reads, cloned from `PlatformWalletInfo.balance`.
    pub(crate) balance: Arc<WalletBalance>,
    /// Shielded (Orchard / ZK) sub-wallet. `None` until [`bind_shielded`]
    /// has run; remains `None` for `WatchOnly` / `ExternalSignable`
    /// wallets that have never had a resolver-driven bind. The
    /// `RwLock` lets the shielded sync coordinator read the bound
    /// state without serializing against unrelated wallet writes.
    ///
    /// [`bind_shielded`]: Self::bind_shielded
    #[cfg(feature = "shielded")]
    pub(crate) shielded: Arc<RwLock<Option<ShieldedWallet<FileBackedShieldedStore>>>>,
}

impl PlatformWallet {
    /// Access the core wallet (balance, UTXOs, addresses).
    pub fn core(&self) -> &CoreWallet<SpvBroadcaster> {
        &self.core
    }

    /// Access the identity wallet.
    ///
    /// Covers both identity-lifecycle and DashPay-contract operations —
    /// these used to be split across `identity()` / `dashpay()`, but the
    /// two facades were merged (the underlying `ManagedIdentity` state
    /// was already shared between them). Keeps the single `SpvBroadcaster`
    /// specialization the rest of this wallet uses.
    pub fn identity(&self) -> &IdentityWallet<SpvBroadcaster> {
        &self.identity
    }

    /// Access the platform address wallet.
    pub fn platform(&self) -> &PlatformAddressWallet {
        &self.platform
    }

    /// Access the shared asset lock manager.
    pub fn asset_locks(&self) -> &Arc<AssetLockManager<SpvBroadcaster>> {
        &self.asset_locks
    }

    /// Get the wallet ID.
    pub fn wallet_id(&self) -> WalletId {
        self.wallet_id
    }

    /// Get a reference to the SDK.
    pub fn sdk(&self) -> &dash_sdk::Sdk {
        &self.sdk
    }

    /// Clone the underlying `Arc<dash_sdk::Sdk>` so callers (e.g. FFI
    /// async blocks moved onto a worker runtime) can hold an
    /// independently-owned SDK handle without keeping the
    /// `PlatformWallet` borrow alive.
    pub fn sdk_arc(&self) -> Arc<dash_sdk::Sdk> {
        Arc::clone(&self.sdk)
    }

    /// Get a reference to the shared wallet manager lock.
    pub fn wallet_manager(&self) -> &Arc<RwLock<WalletManager<PlatformWalletInfo>>> {
        &self.wallet_manager
    }

    /// Get the lock-free balance for UI reads.
    pub fn balance(&self) -> &Arc<WalletBalance> {
        &self.balance
    }

    /// Get a reference to the per-wallet persistence handle.
    ///
    /// Callers that hold a `&PlatformWallet` and need to invoke mutation
    /// methods on [`ManagedIdentity`] (e.g. `set_dashpay_profile`,
    /// `record_dashpay_payment`, `add_identity`) must pass this persister
    /// so those methods can persist the resulting changeset immediately.
    pub fn persister(&self) -> &WalletPersister {
        &self.persister
    }

    /// Read-lock the wallet manager and return a guard that derefs to this
    /// wallet's `PlatformWalletInfo`.
    pub async fn state(&self) -> WalletStateReadGuard<'_> {
        WalletStateReadGuard {
            guard: self.wallet_manager.read().await,
            wallet_id: self.wallet_id,
        }
    }

    /// Write-lock the wallet manager and return a guard that derefs to this
    /// wallet's `PlatformWalletInfo` (with `DerefMut`).
    pub async fn state_mut(&self) -> WalletStateWriteGuard<'_> {
        WalletStateWriteGuard {
            guard: self.wallet_manager.write().await,
            wallet_id: self.wallet_id,
        }
    }

    /// Blocking read-lock.
    pub fn state_blocking(&self) -> WalletStateReadGuard<'_> {
        WalletStateReadGuard {
            guard: self.wallet_manager.blocking_read(),
            wallet_id: self.wallet_id,
        }
    }

    /// Blocking write-lock.
    ///
    /// Uses `tokio::sync::RwLock::blocking_write` — must NOT be
    /// called from within a tokio async context. Exists so sync
    /// callers (e.g. SPV-driven transaction processing) can reach
    /// mutation methods that require the wallet-manager write lock.
    pub fn state_mut_blocking(&self) -> WalletStateWriteGuard<'_> {
        WalletStateWriteGuard {
            guard: self.wallet_manager.blocking_write(),
            wallet_id: self.wallet_id,
        }
    }

    /// Non-blocking read-lock. Returns `None` if the lock is currently
    /// held by a writer, or cannot be acquired without parking the
    /// thread. Safe to call from any context — never panics, never
    /// blocks. Intended for sync callers that run on a tokio runtime
    /// thread (e.g. egui UI render callbacks) where blocking variants
    /// would panic and async variants cannot be awaited.
    pub fn try_state(&self) -> Option<WalletStateReadGuard<'_>> {
        self.wallet_manager
            .try_read()
            .ok()
            .map(|guard| WalletStateReadGuard {
                guard,
                wallet_id: self.wallet_id,
            })
    }

    /// Non-blocking write-lock. Returns `None` if the lock is currently
    /// held by any reader or writer. Same safety properties as
    /// [`try_state`]: never panics, never blocks.
    pub fn try_state_mut(&self) -> Option<WalletStateWriteGuard<'_>> {
        self.wallet_manager
            .try_write()
            .ok()
            .map(|guard| WalletStateWriteGuard {
                guard,
                wallet_id: self.wallet_id,
            })
    }

    /// Construct a PlatformWallet from a WalletManager that already contains
    /// the wallet. The wallet must have been inserted into the WalletManager
    /// before calling this.
    pub(crate) fn new(
        sdk: Arc<dash_sdk::Sdk>,
        wallet_id: WalletId,
        wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        balance: Arc<WalletBalance>,
        lock_notify: Arc<tokio::sync::Notify>,
        persister: Arc<dyn PlatformWalletPersistence>,
        broadcaster: Arc<SpvBroadcaster>,
    ) -> Self {
        // Build the per-wallet persister handle once and share it with
        // the sub-wallets that need to queue their own changesets
        // (currently just `AssetLockManager` — see Item 8 sub-step 1a).
        let wallet_persister = WalletPersister::new(wallet_id, persister);

        let core = CoreWallet::new(
            Arc::clone(&sdk),
            Arc::clone(&wallet_manager),
            wallet_id,
            Arc::clone(&broadcaster),
            Arc::clone(&balance),
        );

        // Asset-lock broadcaster is pinned to `SpvBroadcaster`; the
        // identity wallet's DashPay payment broadcaster uses a clone
        // of the same Arc since production currently runs one
        // broadcaster type across the stack.
        let dashpay_broadcaster = Arc::clone(&broadcaster);

        let asset_locks = Arc::new(AssetLockManager::new(
            Arc::clone(&sdk),
            Arc::clone(&wallet_manager),
            wallet_id,
            lock_notify,
            broadcaster,
            wallet_persister.clone(),
        ));

        let identity: IdentityWallet<SpvBroadcaster> = IdentityWallet {
            sdk: Arc::clone(&sdk),
            wallet_manager: Arc::clone(&wallet_manager),
            wallet_id,
            asset_locks: Arc::clone(&asset_locks),
            persister: wallet_persister.clone(),
            broadcaster: dashpay_broadcaster,
        };

        let platform = PlatformAddressWallet::new(
            Arc::clone(&sdk),
            Arc::clone(&wallet_manager),
            wallet_id,
            wallet_persister.clone(),
        );

        Self {
            wallet_id,
            sdk,
            wallet_manager,
            core,
            identity,
            platform,
            asset_locks,
            persister: wallet_persister,
            balance,
            #[cfg(feature = "shielded")]
            shielded: Arc::new(RwLock::new(None)),
        }
    }

    /// Bind a shielded (Orchard) sub-wallet to this `PlatformWallet`.
    ///
    /// Derives ZIP-32 Orchard keys for every entry of `accounts`
    /// from `seed` (a 32-252 byte BIP-39 seed; see
    /// [`SpendingKey::from_zip32_seed`]), opens or creates the
    /// per-network commitment tree at `db_path`, and stores the
    /// resulting multi-account [`ShieldedWallet`] on this handle.
    /// The caller is responsible for sourcing the seed (e.g. via
    /// the host `MnemonicResolverHandle`) and for zeroizing it
    /// once this call returns. The seed is not retained — only
    /// the per-account FVK / IVK / OVK / default address derived
    /// from it survive on the wallet.
    ///
    /// Idempotent: a second call replaces the previously-bound
    /// shielded wallet (e.g. after a network switch).
    ///
    /// `accounts` must be non-empty; pass `&[0]` for the
    /// single-account default.
    ///
    /// [`SpendingKey::from_zip32_seed`]: grovedb_commitment_tree::SpendingKey::from_zip32_seed
    #[cfg(feature = "shielded")]
    pub async fn bind_shielded(
        &self,
        seed: &[u8],
        accounts: &[u32],
        coordinator: &Arc<crate::wallet::shielded::NetworkShieldedCoordinator>,
    ) -> Result<(), PlatformWalletError> {
        // The store comes from the network-scoped coordinator —
        // every wallet on the same network shares one SQLite
        // handle. The bind also self-registers the wallet's
        // viewing-key set on the coordinator so future sync
        // passes (driven by the coordinator) iterate it.
        // See `PlatformWalletManager::configure_shielded`.
        let store = Arc::clone(coordinator.store());
        let network = self.sdk.network;
        let mut wallet = ShieldedWallet::from_seed_accounts(
            Arc::clone(&self.sdk),
            self.wallet_id,
            seed,
            network,
            accounts,
            store,
        )?;

        // Attach the persister so future sync passes emit
        // shielded changesets the host can mirror (SwiftData
        // on iOS).
        wallet.set_persister(self.persister.clone());

        // Snapshot the viewing-key subset for coordinator
        // registration. Privilege separation: only FVK / IVK /
        // OVK / default address cross to the coordinator; the
        // `SpendAuthorizingKey` stays here on the per-wallet
        // side inside `OrchardKeySet`.
        let account_views: std::collections::BTreeMap<u32, super::shielded::AccountViewingKeys> =
            wallet
                .account_indices()
                .into_iter()
                .filter_map(|account| {
                    wallet
                        .keys_for(account)
                        .ok()
                        .map(|ks| (account, ks.viewing_keys()))
                })
                .collect();

        let mut slot = self.shielded.write().await;
        *slot = Some(wallet);
        drop(slot);

        // Register on the coordinator BEFORE restoring so the
        // restore path's "is this account registered?" gate
        // sees this wallet's subwallets.
        coordinator
            .register_wallet(self.wallet_id, account_views, self.persister.clone())
            .await;

        // Rehydrate per-subwallet notes / sync watermarks from
        // the persister's start state if any are present for
        // this wallet. The lookup is cheap: load() is the
        // boot-time snapshot, indexed by SubwalletId. Errors are
        // logged but not fatal — first-launch wallets simply
        // see no persisted state.
        match self.persister.load() {
            Ok(start) => {
                if let Err(e) = coordinator
                    .restore_for_wallet(self.wallet_id, &start.shielded)
                    .await
                {
                    tracing::warn!(
                        wallet_id = %hex::encode(self.wallet_id),
                        error = %e,
                        "Failed to restore shielded snapshot at bind time"
                    );
                }
            }
            Err(e) => {
                tracing::warn!(
                    wallet_id = %hex::encode(self.wallet_id),
                    error = %e,
                    "persister.load() failed at shielded bind time"
                );
            }
        }
        Ok(())
    }

    /// Add another ZIP-32 account to the already-bound shielded
    /// sub-wallet. Returns `ShieldedNotBound` if `bind_shielded`
    /// hasn't run yet.
    ///
    /// **Caveat**: notes belonging to `account` that already
    /// landed on-chain before the bind call only become spendable
    /// after a tree wipe + re-sync. Hosts that need to discover
    /// historical funds for a freshly-added account should drop
    /// the commitment-tree DB and call [`bind_shielded`] again
    /// with the full account list.
    #[cfg(feature = "shielded")]
    pub async fn shielded_add_account(
        &self,
        seed: &[u8],
        account: u32,
    ) -> Result<(), PlatformWalletError> {
        let mut slot = self.shielded.write().await;
        let wallet = slot.as_mut().ok_or(PlatformWalletError::ShieldedNotBound)?;
        wallet.add_account_from_seed(seed, self.sdk.network, account)
    }

    /// Whether the shielded sub-wallet has been bound via
    /// [`bind_shielded`](Self::bind_shielded).
    #[cfg(feature = "shielded")]
    pub async fn is_shielded_bound(&self) -> bool {
        self.shielded.read().await.is_some()
    }

    /// Bound ZIP-32 account indices on the shielded sub-wallet,
    /// in ascending order. Empty if not bound.
    #[cfg(feature = "shielded")]
    pub async fn shielded_account_indices(&self) -> Vec<u32> {
        self.shielded
            .read()
            .await
            .as_ref()
            .map(|w| w.account_indices())
            .unwrap_or_default()
    }

    /// The default Orchard payment address for `account` on this
    /// wallet, as the raw 43-byte representation. Returns `None`
    /// if the shielded sub-wallet hasn't been bound or `account`
    /// isn't bound on it. Hosts apply their own bech32m encoding
    /// (HRP + 0x10 type byte) on top.
    #[cfg(feature = "shielded")]
    pub async fn shielded_default_address(&self, account: u32) -> Option<[u8; 43]> {
        let guard = self.shielded.read().await;
        guard
            .as_ref()
            .and_then(|w| w.default_address(account).ok())
            .map(|addr| addr.to_raw_address_bytes())
    }

    /// Per-account default Orchard payment addresses (raw 43 bytes).
    #[cfg(feature = "shielded")]
    pub async fn shielded_default_addresses(&self) -> std::collections::BTreeMap<u32, [u8; 43]> {
        let guard = self.shielded.read().await;
        let Some(wallet) = guard.as_ref() else {
            return std::collections::BTreeMap::new();
        };
        wallet
            .account_indices()
            .into_iter()
            .filter_map(|account| {
                wallet
                    .default_address(account)
                    .ok()
                    .map(|addr| (account, addr.to_raw_address_bytes()))
            })
            .collect()
    }

    /// Per-account unspent shielded balance.
    #[cfg(feature = "shielded")]
    pub async fn shielded_balances(
        &self,
    ) -> Result<std::collections::BTreeMap<u32, u64>, PlatformWalletError> {
        let guard = self.shielded.read().await;
        match guard.as_ref() {
            Some(wallet) => wallet.balances().await,
            None => Ok(std::collections::BTreeMap::new()),
        }
    }

    /// Send a private shielded → shielded transfer from `account`'s
    /// notes to `recipient_raw_43` (the recipient's Orchard payment
    /// address as the 43 raw bytes).
    ///
    /// The prover is consumed by value rather than borrowed because
    /// `OrchardProver` is impl'd on `&CachedOrchardProver` (the
    /// reference type), not on the bare struct. Callers pass
    /// `&CachedOrchardProver::new()` and we forward it down to the
    /// underlying `ShieldedWallet::transfer`'s `&P` parameter.
    #[cfg(feature = "shielded")]
    pub async fn shielded_transfer_to<P: dpp::shielded::builder::OrchardProver>(
        &self,
        account: u32,
        recipient_raw_43: &[u8; 43],
        amount: u64,
        prover: P,
    ) -> Result<(), PlatformWalletError> {
        let guard = self.shielded.read().await;
        let shielded = guard
            .as_ref()
            .ok_or(PlatformWalletError::ShieldedNotBound)?;
        let recipient = Option::<grovedb_commitment_tree::PaymentAddress>::from(
            grovedb_commitment_tree::PaymentAddress::from_raw_address_bytes(recipient_raw_43),
        )
        .ok_or_else(|| {
            PlatformWalletError::ShieldedBuildError(
                "invalid Orchard payment address bytes".to_string(),
            )
        })?;
        shielded
            .transfer(account, &recipient, amount, &prover)
            .await
    }

    /// Unshield from `account`'s notes to a transparent platform
    /// address (`"dash1…"` / `"tdash1…"`). Parsed via
    /// `PlatformAddress::from_bech32m_string` and verified against
    /// the wallet's network.
    #[cfg(feature = "shielded")]
    pub async fn shielded_unshield_to<P: dpp::shielded::builder::OrchardProver>(
        &self,
        account: u32,
        to_platform_addr_bech32m: &str,
        amount: u64,
        prover: P,
    ) -> Result<(), PlatformWalletError> {
        let guard = self.shielded.read().await;
        let shielded = guard
            .as_ref()
            .ok_or(PlatformWalletError::ShieldedNotBound)?;
        let (to, addr_network) =
            dpp::address_funds::PlatformAddress::from_bech32m_string(to_platform_addr_bech32m)
                .map_err(|e| {
                    PlatformWalletError::ShieldedBuildError(format!(
                        "invalid platform address: {e}"
                    ))
                })?;
        if addr_network != self.sdk.network {
            return Err(PlatformWalletError::ShieldedBuildError(format!(
                "platform address network mismatch: address {addr_network:?}, wallet {:?}",
                self.sdk.network
            )));
        }
        shielded.unshield(account, &to, amount, &prover).await
    }

    /// Withdraw from `account`'s notes to a Core L1 address
    /// (Base58Check string). `core_fee_per_byte` is the L1 fee
    /// rate (duffs/byte).
    #[cfg(feature = "shielded")]
    pub async fn shielded_withdraw_to<P: dpp::shielded::builder::OrchardProver>(
        &self,
        account: u32,
        to_core_address: &str,
        amount: u64,
        core_fee_per_byte: u32,
        prover: P,
    ) -> Result<(), PlatformWalletError> {
        let guard = self.shielded.read().await;
        let shielded = guard
            .as_ref()
            .ok_or(PlatformWalletError::ShieldedNotBound)?;
        let network = self.sdk.network;
        let parsed = to_core_address
            .parse::<dashcore::Address<dashcore::address::NetworkUnchecked>>()
            .map_err(|e| {
                PlatformWalletError::ShieldedBuildError(format!("invalid core address: {e}"))
            })?
            .require_network(network)
            .map_err(|e| {
                PlatformWalletError::ShieldedBuildError(format!(
                    "core address network mismatch: {e}"
                ))
            })?;
        shielded
            .withdraw(account, &parsed, amount, core_fee_per_byte, &prover)
            .await
    }

    /// Shield credits from a Platform Payment account into the
    /// wallet's shielded pool, with the resulting note assigned
    /// to `shielded_account`'s default Orchard address.
    ///
    /// `payment_account` selects the source Platform Payment
    /// account (different concept from `shielded_account` — this
    /// is the BIP-44-style funding account on the transparent
    /// side, not the ZIP-32 Orchard account). Auto-selects input
    /// addresses from that account in ascending derivation-index
    /// order until the cumulative balance covers `amount` plus a
    /// conservative fee buffer (the on-chain fee comes off input
    /// 0 via `DeductFromInput(0)`; the buffer absorbs the
    /// discrepancy without a more sophisticated estimator).
    ///
    /// The host supplies a `Signer<PlatformAddress>` — typically
    /// `&VTableSigner` from `KeychainSigner.handle` — which signs
    /// each input's pubkey-hash binding to the Orchard bundle.
    ///
    /// Returns `ShieldedNotBound` if no shielded sub-wallet is
    /// bound, `AddressOperation` if the platform-payment account
    /// at `payment_account` doesn't exist, or
    /// `ShieldedInsufficientBalance` if the account's total
    /// credits can't cover `amount + fee_buffer`.
    #[cfg(feature = "shielded")]
    pub async fn shielded_shield_from_account<S, P>(
        &self,
        shielded_account: u32,
        payment_account: u32,
        amount: u64,
        signer: &S,
        prover: P,
    ) -> Result<(), PlatformWalletError>
    where
        S: dpp::identity::signer::Signer<dpp::address_funds::PlatformAddress> + Send + Sync,
        P: dpp::shielded::builder::OrchardProver,
    {
        // The shield transition uses `DeductFromInput(0)` as its fee
        // strategy. drive-abci interprets that as "after each input
        // address has had its `claim` deducted, take the fee out of
        // input 0's *remaining* balance" (see
        // `deduct_fee_from_outputs_or_remaining_balance_of_inputs_v0`
        // in rs-dpp). "Input 0" is the smallest-key entry of the
        // BTreeMap we hand to the builder. Therefore:
        //
        //   * we must NOT claim each input's full balance — claiming
        //     `balance` leaves `remaining = 0`, and the fee
        //     deduction has nothing to bite into.
        //   * we must reserve at least `FEE_RESERVE_CREDITS` of
        //     unclaimed balance specifically on input 0 (the
        //     BTreeMap-smallest address).
        //
        // Empty-mempool fees on Type 15 transitions land at ~20M
        // credits (~0.0002 DASH). Reserve 1e9 credits (0.01 DASH) —
        // 50× headroom, still trivial relative to typical balances.
        const FEE_RESERVE_CREDITS: u64 = 1_000_000_000;

        // Build the inputs map under the wallet-manager read lock,
        // then drop the lock before re-entering shielded so the
        // guards don't nest unnecessarily.
        let inputs: std::collections::BTreeMap<
            dpp::address_funds::PlatformAddress,
            dpp::fee::Credits,
        > = {
            let wm = self.wallet_manager.read().await;
            let info = wm
                .get_wallet_info(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            let account = info
                .core_wallet
                .platform_payment_managed_account_at_index(payment_account)
                .ok_or_else(|| {
                    PlatformWalletError::AddressOperation(format!(
                        "no platform payment account at index {payment_account}"
                    ))
                })?;

            // Collect (address, balance) for every funded address,
            // sorted by address bytes — that determines BTreeMap
            // key order downstream and therefore which input ends
            // up at index 0.
            let mut candidates: Vec<(dpp::address_funds::PlatformAddress, u64)> = account
                .addresses
                .addresses
                .values()
                .filter_map(|addr_info| {
                    let p2pkh =
                        key_wallet::PlatformP2PKHAddress::from_address(&addr_info.address).ok()?;
                    let balance = account.address_credit_balance(&p2pkh);
                    if balance == 0 {
                        None
                    } else {
                        Some((
                            dpp::address_funds::PlatformAddress::P2pkh(p2pkh.to_bytes()),
                            balance,
                        ))
                    }
                })
                .collect();
            candidates.sort_by_key(|(addr, _)| *addr);

            // The address that will be the bundle's `input_0` must
            // have balance > FEE_RESERVE so we can claim at least 1
            // credit while leaving the reserve untouched. Skip any
            // leading dust address that can't satisfy that — the
            // next address up will become input 0 instead. If
            // every funded address is below the reserve, fail fast:
            // the network would reject the broadcast on the
            // boundary anyway, only after we've spent ~30 s
            // building the Halo 2 proof.
            let Some(viable_input_0) = candidates
                .iter()
                .position(|(_, balance)| *balance > FEE_RESERVE_CREDITS)
            else {
                let total: u64 = candidates.iter().map(|(_, b)| b).sum();
                return Err(PlatformWalletError::ShieldedInsufficientBalance {
                    available: total,
                    required: amount.saturating_add(FEE_RESERVE_CREDITS),
                });
            };
            let usable: &[(dpp::address_funds::PlatformAddress, u64)] =
                &candidates[viable_input_0..];

            let total_usable: u64 = usable.iter().map(|(_, b)| b).sum();
            let needed = amount.saturating_add(FEE_RESERVE_CREDITS);
            if total_usable < needed {
                return Err(PlatformWalletError::ShieldedInsufficientBalance {
                    available: total_usable,
                    required: needed,
                });
            }

            // Walk usable inputs in BTreeMap order, claiming only
            // what's needed to cover `amount`. The fee reserve is
            // taken off input 0's max claim so its post-claim
            // remaining stays ≥ FEE_RESERVE_CREDITS for the
            // network's `DeductFromInput(0)` step.
            let mut chosen: std::collections::BTreeMap<
                dpp::address_funds::PlatformAddress,
                dpp::fee::Credits,
            > = std::collections::BTreeMap::new();
            let mut accumulated_claim: u64 = 0;
            for (i, (addr, balance)) in usable.iter().enumerate() {
                if accumulated_claim >= amount {
                    break;
                }
                let max_claim = if i == 0 {
                    balance.saturating_sub(FEE_RESERVE_CREDITS)
                } else {
                    *balance
                };
                let still_need = amount - accumulated_claim;
                let claim = max_claim.min(still_need);
                if claim > 0 {
                    chosen.insert(*addr, claim);
                    accumulated_claim = accumulated_claim.saturating_add(claim);
                }
            }

            if accumulated_claim < amount {
                return Err(PlatformWalletError::ShieldedInsufficientBalance {
                    available: accumulated_claim,
                    required: amount,
                });
            }
            chosen
        };

        let guard = self.shielded.read().await;
        let shielded = guard
            .as_ref()
            .ok_or(PlatformWalletError::ShieldedNotBound)?;
        shielded
            .shield(shielded_account, inputs, amount, signer, &prover)
            .await
    }
}

impl PlatformWallet {
    // TODO: What these methods for? can we remove? Don't deelete this todo
    /// Queue a changeset for later persistence.
    pub fn queue_persist(&self, changeset: PlatformWalletChangeSet) {
        if let Err(e) = self.persister.store(changeset) {
            tracing::error!(
                error = %e,
                wallet_id = %hex::encode(self.wallet_id),
                "Failed to queue changeset for persistence"
            );
        }
    }

    /// Flush all queued changesets to the storage backend.
    pub fn flush_persist(&self) -> Result<(), PersistenceError> {
        self.persister.flush()
    }

    /// Load persisted state for this wallet.
    pub fn load_persisted(&self) -> Result<ClientStartState, PersistenceError> {
        self.persister.load()
    }

    /// Apply a [`PlatformWalletChangeSet`] to this wallet's in-memory
    /// state under the wallet manager write lock.
    ///
    /// Delegates to [`PlatformWalletInfo::apply_changeset`], which is
    /// the canonical restore path. Holds the `WalletManager` write
    /// lock for the duration so the split borrow of `(&mut Wallet,
    /// &mut PlatformWalletInfo)` is safe — `&mut Wallet` is needed so
    /// the core sub-changeset can re-derive HD accounts via
    /// `Wallet::add_account`.
    ///
    /// Returns [`ApplyError::WalletNotFound`](crate::wallet::ApplyError::WalletNotFound)
    /// if the wallet has been removed from the manager between handle
    /// acquisition and this call.
    ///
    /// Consumes the changeset by value — `apply_changeset` drains
    /// every map straight into the wallet maps with no clones.
    pub async fn apply(
        &self,
        changeset: PlatformWalletChangeSet,
    ) -> Result<(), crate::wallet::ApplyError> {
        // The platform-address sync watermark lives on the provider,
        // not on `PlatformWalletInfo`. Pull it out before handing the
        // changeset to `apply_changeset` (which consumes by value), then
        // feed it to the providers once apply completes.
        let pa_sync_state = changeset.platform_addresses.as_ref().map(|pa| {
            (
                pa.sync_height,
                pa.sync_timestamp,
                pa.last_known_recent_block,
            )
        });

        {
            let mut wm = self.wallet_manager.write().await;
            let (wallet, info) = wm
                .get_wallet_mut_and_info_mut(&self.wallet_id)
                .ok_or(crate::wallet::ApplyError::WalletNotFound(self.wallet_id))?;
            info.apply_changeset(wallet, changeset)?;
        }

        if let Some((height, timestamp, recent_block)) = pa_sync_state {
            self.platform
                .apply_sync_state(height, timestamp, recent_block)
                .await;
        }
        Ok(())
    }

    /// Load persisted state from the persister and apply it to the
    /// in-memory wallet. Convenience wrapper for
    /// `apply(load_persisted()?)`.
    ///
    /// **Idempotent** — safe to call multiple times. The apply path
    /// uses monotonic / OR merges on every field it touches
    /// (`highest_used` is MAX-merged, `utxos_instant_locked` is
    /// set-union), so re-applying the same persisted state is a no-op.
    ///
    /// This is the recommended entry point for startup hydration
    /// *after* late-registered accounts (e.g. DashPay contact
    /// accounts that `bootstrap_dashpay_contact_accounts` adds) have
    /// landed. The initial load_and_apply called during
    /// `PlatformWallet` construction only hydrates state for
    /// accounts that exist at that point; a second call after
    /// account bootstrap picks up the rest without regressing
    /// anything.
    pub async fn load_and_apply_persisted(
        &self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let ClientStartState {
            mut platform_addresses,
            wallets: _,
            #[cfg(feature = "shielded")]
                shielded: _,
        } = self.load_persisted()?;

        if let Some(persisted) = platform_addresses.remove(&self.wallet_id) {
            self.platform
                .initialize_from_persisted(persisted)
                .await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        }

        Ok(())
    }
}

impl Clone for PlatformWallet {
    fn clone(&self) -> Self {
        Self {
            wallet_id: self.wallet_id,
            sdk: self.sdk.clone(),
            wallet_manager: self.wallet_manager.clone(),
            core: self.core.clone(),
            identity: self.identity.clone(),
            platform: self.platform.clone(),
            asset_locks: self.asset_locks.clone(),
            persister: self.persister.clone(),
            balance: self.balance.clone(),
            #[cfg(feature = "shielded")]
            shielded: self.shielded.clone(),
        }
    }
}

impl std::fmt::Debug for PlatformWallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlatformWallet")
            .field("wallet_id", &hex::encode(self.wallet_id))
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Wallet state guard types — lock WalletManager, deref to PlatformWalletInfo
// ---------------------------------------------------------------------------

/// Read guard that locks `WalletManager` and derefs to this wallet's
/// `PlatformWalletInfo`. Also provides `.wallet()` for key material access.
pub struct WalletStateReadGuard<'a> {
    guard: RwLockReadGuard<'a, WalletManager<PlatformWalletInfo>>,
    wallet_id: WalletId,
}

impl<'a> WalletStateReadGuard<'a> {
    /// Access the immutable `Wallet` (key material).
    pub fn wallet(&self) -> &Wallet {
        self.guard
            .get_wallet(&self.wallet_id)
            .expect("wallet exists in guard")
    }
}

impl Deref for WalletStateReadGuard<'_> {
    type Target = PlatformWalletInfo;
    fn deref(&self) -> &PlatformWalletInfo {
        self.guard
            .get_wallet_info(&self.wallet_id)
            .expect("wallet exists in guard")
    }
}

/// Write guard that locks `WalletManager` and derefs to this wallet's
/// `PlatformWalletInfo` (with `DerefMut`). Also provides `.wallet()`.
pub struct WalletStateWriteGuard<'a> {
    guard: RwLockWriteGuard<'a, WalletManager<PlatformWalletInfo>>,
    wallet_id: WalletId,
}

impl<'a> WalletStateWriteGuard<'a> {
    /// Access the immutable `Wallet` (key material).
    pub fn wallet(&self) -> &Wallet {
        self.guard
            .get_wallet(&self.wallet_id)
            .expect("wallet exists in guard")
    }
}

impl Deref for WalletStateWriteGuard<'_> {
    type Target = PlatformWalletInfo;
    fn deref(&self) -> &PlatformWalletInfo {
        self.guard
            .get_wallet_info(&self.wallet_id)
            .expect("wallet exists in guard")
    }
}

impl DerefMut for WalletStateWriteGuard<'_> {
    fn deref_mut(&mut self) -> &mut PlatformWalletInfo {
        self.guard
            .get_wallet_info_mut(&self.wallet_id)
            .expect("wallet exists in guard")
    }
}
