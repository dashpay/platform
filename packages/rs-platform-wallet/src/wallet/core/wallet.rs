//! Core wallet functionality: balance, UTXOs, addresses, transaction history.

use std::sync::Arc;

use super::balance::WalletBalance;

use dashcore::Address as DashAddress;
use tokio::sync::RwLock;

use key_wallet::managed_account::address_pool::KeySource;
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::wallet::managed_wallet_info::transaction_building::AccountTypePreference;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet_manager::WalletManager;

use crate::broadcaster::TransactionBroadcaster;
use crate::error::PlatformWalletError;
use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};

/// Core wallet providing UTXO, balance, and address functionality.
///
/// This is a lightweight handle — all mutable state lives in the shared
/// `WalletManager<PlatformWalletInfo>` behind an `Arc<RwLock<…>>`.
/// The handle holds `Arc` references and is cheaply `Clone`able.
///
/// `B` is the concrete transaction-broadcaster type. The generic
/// parameter lets broadcast calls dispatch statically instead of
/// through a `dyn` vtable.
pub struct CoreWallet<B: TransactionBroadcaster + ?Sized> {
    pub(crate) sdk: Arc<dash_sdk::Sdk>,
    pub(crate) wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    pub(crate) wallet_id: WalletId,
    /// Injected broadcaster — delegates to SPV or DAPI depending on how
    /// the wallet was constructed by `PlatformWalletManager`.
    pub(crate) broadcaster: Arc<B>,
    /// Lock-free balance for UI reads.
    balance: Arc<WalletBalance>,
}

impl<B: TransactionBroadcaster + ?Sized> CoreWallet<B> {
    pub(crate) fn new(
        sdk: Arc<dash_sdk::Sdk>,
        wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        wallet_id: WalletId,
        broadcaster: Arc<B>,
        balance: Arc<WalletBalance>,
    ) -> Self {
        Self {
            sdk,
            wallet_manager,
            wallet_id,
            broadcaster,
            balance,
        }
    }

    /// Lock-free balance snapshot for UI reads.
    pub fn balance(&self) -> &WalletBalance {
        &self.balance
    }

    /// Wallet id this `CoreWallet` operates on. Exposed so FFI
    /// callers that need to construct a per-call `Signer` (e.g.
    /// `MnemonicResolverCoreSigner`) can thread the same wallet id
    /// the resolver callback will receive.
    pub fn wallet_id(&self) -> WalletId {
        self.wallet_id
    }

    /// Whether `self` and `other` are handles to the same wallet *generation* —
    /// the same logical wallet AND the same live in-memory instance.
    ///
    /// Two aliases of one generation (the `Arc<PlatformWallet>` clones handed
    /// out by `PlatformWalletManager::get_wallet`) share the per-generation
    /// `Arc<WalletBalance>`; a wallet removed and re-created under the same
    /// `wallet_id` gets a fresh one. `Arc::ptr_eq` on that balance therefore
    /// distinguishes generations that `wallet_id` — and the shared multi-wallet
    /// `WalletManager` `Arc` — alone cannot (both are equal across a
    /// remove-then-recreate). While either handle is held the balance `Arc`
    /// cannot be freed, so its address can never be reused for a different
    /// generation, which makes the pointer comparison sound (the same soundness
    /// argument the registry already relies on for `Arc::ptr_eq` on the
    /// manager).
    ///
    /// This is the single generation identity shared by BOTH deferred-payment
    /// paths — the registry-token path
    /// ([`SignedPaymentRegistry`](crate::SignedPaymentRegistry), `dashpay/platform#4185`)
    /// and the V2 finalized-transaction handle path (`dashpay/platform#4196`) —
    /// so neither acts on a re-created wallet's `ReservationSet` while an old
    /// handle still names the old generation.
    pub fn is_same_generation<O: TransactionBroadcaster + ?Sized>(
        &self,
        other: &CoreWallet<O>,
    ) -> bool {
        self.wallet_id == other.wallet_id
            && Arc::ptr_eq(&self.wallet_manager, &other.wallet_manager)
            && Arc::ptr_eq(&self.balance, &other.balance)
    }

    /// This handle's per-generation balance `Arc` — the generation-identity
    /// marker (see [`is_same_generation`](Self::is_same_generation)). The
    /// manager stores the same `Arc` in `PlatformWalletInfo.balance`, so a
    /// reservation-cleanup path can, **under the manager lock**, compare this
    /// against the wallet currently registered under `wallet_id` and act only if
    /// they are the same generation — binding a validate-then-mutate to one lock
    /// hold and refusing to touch a generation re-created under the same id.
    pub(crate) fn generation(&self) -> &Arc<WalletBalance> {
        &self.balance
    }

    /// This handle's per-generation identity marker, cloned — for tests (and
    /// downstream FFI-crate tests via `test-utils`) that build a finalized
    /// [`SignedCoreTransaction`](crate::SignedCoreTransaction) with
    /// [`new_for_test`](crate::SignedCoreTransaction::new_for_test) and must
    /// stamp it with the SAME generation they then register it against, exactly
    /// as the production `finalize_transaction` path binds a token to the
    /// finalizing wallet.
    #[cfg(any(test, feature = "test-utils"))]
    pub fn test_generation_marker(&self) -> Arc<WalletBalance> {
        Arc::clone(&self.balance)
    }

    pub async fn set_gap_limit(
        &self,
        account_type: AccountTypePreference,
        account_index: u32,
        gap_limit: u32,
    ) -> Result<(), PlatformWalletError> {
        let mut wm = self.wallet_manager.write().await;
        let (wallet, info) = wm.get_wallet_and_info_mut(&self.wallet_id).ok_or_else(|| {
            PlatformWalletError::WalletNotFound("Wallet not found in wallet manager".to_string())
        })?;

        let xpub = match account_type {
            AccountTypePreference::BIP44 => wallet.get_bip44_account(account_index),
            AccountTypePreference::BIP32 => wallet.get_bip32_account(account_index),
            AccountTypePreference::CoinJoin => wallet.get_coinjoin_account(account_index),
        }
        .map(|a| a.account_xpub)
        .ok_or_else(|| {
            PlatformWalletError::WalletNotFound(format!(
                "wallet account {account_type:?} #{account_index} not found"
            ))
        })?;

        let account = match account_type {
            AccountTypePreference::BIP44 => info
                .core_wallet
                .accounts
                .standard_bip44_accounts
                .get_mut(&account_index),
            AccountTypePreference::BIP32 => info
                .core_wallet
                .accounts
                .standard_bip32_accounts
                .get_mut(&account_index),
            AccountTypePreference::CoinJoin => info
                .core_wallet
                .accounts
                .coinjoin_accounts
                .get_mut(&account_index),
        }
        .ok_or_else(|| {
            PlatformWalletError::WalletNotFound(format!(
                "managed account {account_type:?} #{account_index} not found"
            ))
        })?;

        account
            .set_gap_limit(gap_limit, &KeySource::Public(xpub))
            .map(|_| ())
            .map_err(|e| PlatformWalletError::AddressOperation(e.to_string()))
    }

    /// Get the next unused BIP-44 external (receive) address for a specific account.
    pub async fn next_receive_address_for_account(
        &self,
        account_index: u32,
    ) -> Result<DashAddress, PlatformWalletError> {
        let mut wm = self.wallet_manager.write().await;
        let (wallet, info) = wm.get_wallet_and_info_mut(&self.wallet_id).ok_or_else(|| {
            crate::error::PlatformWalletError::WalletNotFound(
                "Wallet not found in wallet manager".to_string(),
            )
        })?;

        let xpub = wallet
            .accounts
            .standard_bip44_accounts
            .get(&account_index)
            .map(|a| a.account_xpub)
            .ok_or_else(|| {
                PlatformWalletError::WalletNotFound(format!(
                    "BIP-44 account {} not found",
                    account_index
                ))
            })?;

        let account = info
            .core_wallet
            .accounts
            .standard_bip44_accounts
            .get_mut(&account_index)
            .ok_or_else(|| {
                PlatformWalletError::WalletNotFound(format!(
                    "BIP-44 managed account {} not found",
                    account_index
                ))
            })?;

        account
            .next_receive_address(Some(&xpub), true)
            .map_err(|e| PlatformWalletError::AddressOperation(e.to_string()))
    }

    /// Blocking version of `next_receive_address_for_account`.
    pub fn next_receive_address_for_account_blocking(
        &self,
        account_index: u32,
    ) -> Result<DashAddress, PlatformWalletError> {
        let mut wm = self.wallet_manager.blocking_write();
        let (wallet, info) = wm.get_wallet_and_info_mut(&self.wallet_id).ok_or_else(|| {
            crate::error::PlatformWalletError::WalletNotFound(
                "Wallet not found in wallet manager".to_string(),
            )
        })?;

        let xpub = wallet
            .accounts
            .standard_bip44_accounts
            .get(&account_index)
            .map(|a| a.account_xpub)
            .ok_or_else(|| {
                PlatformWalletError::WalletNotFound(format!(
                    "BIP-44 account {} not found",
                    account_index
                ))
            })?;

        let account = info
            .core_wallet
            .accounts
            .standard_bip44_accounts
            .get_mut(&account_index)
            .ok_or_else(|| {
                PlatformWalletError::WalletNotFound(format!(
                    "BIP-44 managed account {} not found",
                    account_index
                ))
            })?;

        account
            .next_receive_address(Some(&xpub), true)
            .map_err(|e| PlatformWalletError::AddressOperation(e.to_string()))
    }

    /// Get the next unused BIP-44 internal (change) address for a specific account.
    pub async fn next_change_address_for_account(
        &self,
        account_index: u32,
    ) -> Result<DashAddress, PlatformWalletError> {
        let mut wm = self.wallet_manager.write().await;
        let (wallet, info) = wm.get_wallet_and_info_mut(&self.wallet_id).ok_or_else(|| {
            crate::error::PlatformWalletError::WalletNotFound(
                "Wallet not found in wallet manager".to_string(),
            )
        })?;

        let xpub = wallet
            .accounts
            .standard_bip44_accounts
            .get(&account_index)
            .map(|a| a.account_xpub)
            .ok_or_else(|| {
                PlatformWalletError::WalletNotFound(format!(
                    "BIP-44 account {} not found",
                    account_index
                ))
            })?;

        let account = info
            .core_wallet
            .accounts
            .standard_bip44_accounts
            .get_mut(&account_index)
            .ok_or_else(|| {
                PlatformWalletError::WalletNotFound(format!(
                    "BIP-44 managed account {} not found",
                    account_index
                ))
            })?;

        account
            .next_change_address(Some(&xpub), true)
            .map_err(|e| PlatformWalletError::AddressOperation(e.to_string()))
    }

    /// Blocking version of `next_change_address_for_account`.
    pub fn next_change_address_for_account_blocking(
        &self,
        account_index: u32,
    ) -> Result<DashAddress, PlatformWalletError> {
        let mut wm = self.wallet_manager.blocking_write();
        let (wallet, info) = wm.get_wallet_and_info_mut(&self.wallet_id).ok_or_else(|| {
            crate::error::PlatformWalletError::WalletNotFound(
                "Wallet not found in wallet manager".to_string(),
            )
        })?;

        let xpub = wallet
            .accounts
            .standard_bip44_accounts
            .get(&account_index)
            .map(|a| a.account_xpub)
            .ok_or_else(|| {
                PlatformWalletError::WalletNotFound(format!(
                    "BIP-44 account {} not found",
                    account_index
                ))
            })?;

        let account = info
            .core_wallet
            .accounts
            .standard_bip44_accounts
            .get_mut(&account_index)
            .ok_or_else(|| {
                PlatformWalletError::WalletNotFound(format!(
                    "BIP-44 managed account {} not found",
                    account_index
                ))
            })?;

        account
            .next_change_address(Some(&xpub), true)
            .map_err(|e| PlatformWalletError::AddressOperation(e.to_string()))
    }

    /// Get the network from the SDK.
    pub fn network(&self) -> key_wallet::Network {
        self.sdk.network
    }

    /// Current last-processed block height for this wallet, or `None` if the
    /// wallet is no longer present in the manager.
    ///
    /// This is the clock the funding reservation is actually stamped with:
    /// `finalize_transaction` / `build_signed` reserve the selected inputs at
    /// `set_current_height(last_processed_height())`, and key-wallet's
    /// `ReservationSet` TTL sweeps entries relative to a later build's
    /// `last_processed_height`. It is therefore the correct — and monotonic —
    /// clock for the deferred-payment
    /// [`SignedPaymentRegistry`](crate::SignedPaymentRegistry) to bound a token's
    /// lifetime against that TTL. `synced_height` is a different clock that can
    /// regress during a rescan, so measuring the reservation's age against it
    /// could let a token outlive its reservation.
    pub(crate) async fn last_processed_height(&self) -> Option<u32> {
        let wm = self.wallet_manager.read().await;
        wm.get_wallet_and_info(&self.wallet_id)
            .map(|(_, info)| info.core_wallet.last_processed_height())
    }

    /// Whether the generation this handle names is STILL the one registered
    /// under its `wallet_id` in the manager.
    ///
    /// [`is_same_generation`](Self::is_same_generation) compares two *handles*
    /// and therefore cannot see either way a generation stops being current:
    ///
    /// * **Removed** (`platform_wallet_manager_remove_wallet`). A retained
    ///   handle keeps `wallet_id`, the shared manager `Arc`, and its own balance
    ///   `Arc` alive, so two handles to the removed generation still compare
    ///   equal to each other. Only a lookup against the manager can tell that
    ///   nothing is registered under the id any more.
    /// * **Re-created** under the same id. `wallet_id` and the manager `Arc` are
    ///   preserved; only the balance `Arc` is fresh.
    ///
    /// Both cases mean the same thing to a deferred payment: the accounts —
    /// and therefore the `ReservationSet` holding its funding inputs — that this
    /// handle names are no longer the wallet's live state, so acting on them
    /// would spend against state the manager no longer owns. Callers that must
    /// be atomic against a concurrent teardown take
    /// [`SignedPaymentRegistry::lifecycle_read`](crate::SignedPaymentRegistry::lifecycle_read)
    /// around the check and the action it gates; on its own this is a point-in-
    /// time observation (`dashpay/platform#4185`).
    pub async fn is_current_generation(&self) -> bool {
        let wm = self.wallet_manager.read().await;
        wm.get_wallet_info(&self.wallet_id)
            .is_some_and(|info| Arc::ptr_eq(&info.balance, self.generation()))
    }
}

impl<B: TransactionBroadcaster + ?Sized> std::fmt::Debug for CoreWallet<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CoreWallet")
            .field("network", &self.sdk.network)
            .finish()
    }
}

// Manual `Clone` impl: the derive would add a `where B: Clone`
// bound, but `Arc<B>` clones without cloning `B` itself, so we
// don't want that bound. `B: ?Sized` is enough.
impl<B: TransactionBroadcaster + ?Sized> Clone for CoreWallet<B> {
    fn clone(&self) -> Self {
        Self {
            sdk: Arc::clone(&self.sdk),
            wallet_manager: Arc::clone(&self.wallet_manager),
            wallet_id: self.wallet_id,
            broadcaster: Arc::clone(&self.broadcaster),
            balance: Arc::clone(&self.balance),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use key_wallet::account::account_type::StandardAccountType;

    use super::WalletBalance;
    use crate::test_support::{funded_wallet_manager, AlwaysOkBroadcaster};
    use crate::wallet::core::CoreWallet;

    /// The single generation identity both deferred-payment paths share:
    /// aliases of one generation share the per-generation balance `Arc` (same
    /// generation), while a wallet re-created under the same `wallet_id` and the
    /// same multi-wallet `WalletManager` `Arc` but a fresh balance `Arc` is a
    /// DIFFERENT generation. Neither `wallet_id` nor the manager `Arc` alone can
    /// tell them apart — the balance `Arc` is what distinguishes them, closing
    /// the gap where an old handle could act through the old generation while a
    /// new generation selected the same inputs.
    #[tokio::test]
    async fn is_same_generation_distinguishes_recreation_from_aliases() {
        let (manager, wallet_id, balance, _signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let broadcaster = Arc::new(AlwaysOkBroadcaster);

        let generation_a = CoreWallet::new(
            Arc::clone(&sdk),
            Arc::clone(&manager),
            wallet_id,
            Arc::clone(&broadcaster),
            Arc::clone(&balance),
        );

        // A clone is an alias of the SAME generation (shares the balance Arc).
        let alias = generation_a.clone();
        assert!(
            generation_a.is_same_generation(&alias),
            "aliases of one generation must compare equal"
        );
        assert!(alias.is_same_generation(&generation_a));

        // A re-created generation: SAME manager Arc + SAME wallet_id, fresh
        // per-generation balance Arc.
        let generation_b = CoreWallet::new(
            sdk,
            Arc::clone(&manager),
            wallet_id,
            broadcaster,
            Arc::new(WalletBalance::new()),
        );
        assert!(
            !generation_a.is_same_generation(&generation_b),
            "a re-created generation must NOT match, despite equal wallet_id + manager"
        );
        // Sanity: it is ONLY the balance Arc that differs — wallet_id and the
        // manager Arc are identical, so those checks alone could not tell the
        // two generations apart.
        assert_eq!(generation_a.wallet_id(), generation_b.wallet_id());
        assert!(Arc::ptr_eq(
            &generation_a.wallet_manager,
            &generation_b.wallet_manager
        ));
    }
}
