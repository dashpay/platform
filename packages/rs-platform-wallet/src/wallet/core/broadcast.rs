use dashcore::{Address as DashAddress, Transaction};
use key_wallet::account::account_type::StandardAccountType;
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::managed_account::ManagedCoreFundsAccount;
use key_wallet::signer::Signer;

use crate::broadcaster::TransactionBroadcaster;
use crate::wallet::platform_wallet::PlatformWalletInfo;
use crate::{CoreWallet, PlatformWalletError};

/// Mutable funds account for `account_type`/`account_index`, or `None` if it
/// is not present.
fn funds_account_mut(
    info: &mut PlatformWalletInfo,
    account_type: StandardAccountType,
    account_index: u32,
) -> Option<&mut ManagedCoreFundsAccount> {
    match account_type {
        StandardAccountType::BIP44Account => info
            .core_wallet
            .accounts
            .standard_bip44_accounts
            .get_mut(&account_index),
        StandardAccountType::BIP32Account => info
            .core_wallet
            .accounts
            .standard_bip32_accounts
            .get_mut(&account_index),
    }
}

impl<B: TransactionBroadcaster + ?Sized> CoreWallet<B> {
    /// Broadcast a signed transaction to the network.
    ///
    /// Build the transaction using key-wallet's
    /// [`TransactionBuilder`](key_wallet::wallet::managed_wallet_info::transaction_builder::TransactionBuilder),
    /// then pass the result here for broadcasting.
    ///
    /// Delegates to the injected [`TransactionBroadcaster`] which may use
    /// SPV (P2P) or DAPI (gRPC) depending on how the wallet was constructed.
    ///
    /// Returns the transaction ID on success.
    pub async fn broadcast_transaction(
        &self,
        transaction: &Transaction,
    ) -> Result<dashcore::Txid, PlatformWalletError> {
        self.broadcaster.broadcast(transaction).await
    }

    /// Build, sign, and broadcast a payment to the given addresses.
    ///
    /// Uses key-wallet's
    /// [`TransactionBuilder`](key_wallet::wallet::managed_wallet_info::transaction_builder::TransactionBuilder)
    /// for UTXO selection, fee estimation, and signing. Change is sent to
    /// the next internal address of the specified account.
    ///
    /// Signing is delegated to the caller-supplied
    /// [`Signer`](key_wallet::signer::Signer) via the
    /// `impl<S: Signer> TransactionSigner for S` blanket in
    /// `key-wallet`'s `transaction_builder.rs`. For Swift wallets this
    /// is typically a
    /// [`MnemonicResolverCoreSigner`](crate::wallet::asset_lock::build)
    /// from `platform-wallet-ffi`, backed by the Keychain-resolver
    /// vtable so private keys never cross the FFI boundary.
    ///
    /// **Note (smell):** the body of this method is a near-duplicate of
    /// `ManagedWalletInfo::build_and_sign_transaction` in `key-wallet`
    /// (`wallet/managed_wallet_info/transaction_building.rs`).
    /// It's reimplemented here because the upstream helper is BIP-44-only,
    /// parametrizing upstream on `AccountTypePreference` so it picks
    /// `standard_bip{32,44}_accounts` would be a trivial change
    pub async fn send_to_addresses<S: Signer>(
        &self,
        account_type: StandardAccountType,
        account_index: u32,
        outputs: Vec<(DashAddress, u64)>,
        signer: &S,
    ) -> Result<Transaction, PlatformWalletError> {
        use key_wallet::wallet::managed_wallet_info::coin_selection::SelectionStrategy;
        use key_wallet::wallet::managed_wallet_info::transaction_builder::TransactionBuilder;
        use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;

        if outputs.is_empty() {
            return Err(PlatformWalletError::TransactionBuild(
                "No outputs specified".to_string(),
            ));
        }

        let tx = {
            let mut wm = self.wallet_manager.write().await;
            let (wallet, info) = wm.get_wallet_and_info_mut(&self.wallet_id).ok_or_else(|| {
                crate::error::PlatformWalletError::WalletNotFound(
                    "Wallet not found in wallet manager".to_string(),
                )
            })?;

            let current_height = info.core_wallet.synced_height();

            let (managed_account, account) = match account_type {
                StandardAccountType::BIP44Account => (
                    info.core_wallet
                        .accounts
                        .standard_bip44_accounts
                        .get_mut(&account_index)
                        .ok_or_else(|| {
                            PlatformWalletError::TransactionBuild(format!(
                                "{:?} managed account {} not found",
                                account_type, account_index
                            ))
                        })?,
                    wallet
                        .accounts
                        .standard_bip44_accounts
                        .get(&account_index)
                        .ok_or_else(|| {
                            PlatformWalletError::TransactionBuild(format!(
                                "{:?} account {} not found in wallet",
                                account_type, account_index
                            ))
                        })?,
                ),
                StandardAccountType::BIP32Account => (
                    info.core_wallet
                        .accounts
                        .standard_bip32_accounts
                        .get_mut(&account_index)
                        .ok_or_else(|| {
                            PlatformWalletError::TransactionBuild(format!(
                                "{:?} managed account {} not found",
                                account_type, account_index
                            ))
                        })?,
                    wallet
                        .accounts
                        .standard_bip32_accounts
                        .get(&account_index)
                        .ok_or_else(|| {
                            PlatformWalletError::TransactionBuild(format!(
                                "{:?} account {} not found in wallet",
                                account_type, account_index
                            ))
                        })?,
                ),
            };

            // The blanket `impl<S: Signer> TransactionSigner for S` in
            // `key-wallet/src/wallet/managed_wallet_info/transaction_builder.rs:482`
            // makes the signer drop-in for the previously `Wallet`-backed
            // path; the funds-derived `address_derivation_path` lookup is
            // unchanged.
            let mut builder = TransactionBuilder::new()
                .set_current_height(current_height)
                .set_selection_strategy(SelectionStrategy::LargestFirst)
                .set_funding(managed_account, account);
            for (addr, amount) in &outputs {
                builder = builder.add_output(addr, *amount);
            }

            let (tx, _fee) = builder
                .build_signed(signer, |addr| {
                    managed_account.address_derivation_path(&addr)
                })
                .await
                .map_err(|e| PlatformWalletError::TransactionBuild(e.to_string()))?;
            tx
        };

        if let Err(e) = self.broadcast_transaction(&tx).await {
            // Abandoned build: release its reserved inputs now so an immediate
            // retry can reselect them, instead of stranding the coins until the
            // 24-block TTL backstop. A successful broadcast releases on its own
            // once the spend is processed back into the wallet.
            let mut wm = self.wallet_manager.write().await;
            match wm.get_wallet_and_info_mut(&self.wallet_id) {
                Some((_, info)) => match funds_account_mut(info, account_type, account_index) {
                    Some(managed_account) => managed_account.release_reservation(&tx),
                    None => tracing::warn!(
                        ?account_type,
                        account_index,
                        "could not release UTXO reservation after failed broadcast: \
                         funds account not found"
                    ),
                },
                None => tracing::warn!(
                    "could not release UTXO reservation after failed broadcast: wallet not found"
                ),
            }
            return Err(e);
        }
        Ok(tx)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    use async_trait::async_trait;
    use dashcore::secp256k1::{ecdsa, Message, PublicKey, Secp256k1};
    use dashcore::{Address as DashAddress, Network, Transaction, Txid};
    use key_wallet::account::account_type::StandardAccountType;
    use key_wallet::signer::{Signer, SignerMethod};
    use key_wallet::test_utils::TestWalletContext;
    use key_wallet::{DerivationPath, Wallet};
    use key_wallet_manager::WalletManager;
    use tokio::sync::RwLock;

    use crate::broadcaster::TransactionBroadcaster;
    use crate::wallet::core::{CoreWallet, WalletBalance};
    use crate::wallet::identity::IdentityManager;
    use crate::wallet::platform_wallet::PlatformWalletInfo;
    use crate::PlatformWalletError;

    /// Broadcaster that fails its first call and succeeds afterwards, to model
    /// a transient broadcast error followed by a user retry.
    struct FailFirstBroadcaster {
        failed_once: AtomicBool,
    }

    impl FailFirstBroadcaster {
        fn new() -> Self {
            Self {
                failed_once: AtomicBool::new(false),
            }
        }
    }

    #[async_trait]
    impl TransactionBroadcaster for FailFirstBroadcaster {
        async fn broadcast(&self, transaction: &Transaction) -> Result<Txid, PlatformWalletError> {
            if self.failed_once.swap(true, Ordering::SeqCst) {
                Ok(transaction.txid())
            } else {
                Err(PlatformWalletError::TransactionBroadcast(
                    "simulated broadcast failure".to_string(),
                ))
            }
        }
    }

    /// Soft signer that derives keys straight from a test wallet's seed. Stands
    /// in for the FFI keychain-backed signer used in production.
    struct WalletSigner {
        wallet: Wallet,
    }

    #[async_trait]
    impl Signer for WalletSigner {
        type Error = String;

        fn supported_methods(&self) -> &[SignerMethod] {
            &[SignerMethod::Digest]
        }

        async fn sign_ecdsa(
            &self,
            path: &DerivationPath,
            sighash: [u8; 32],
        ) -> Result<(ecdsa::Signature, PublicKey), Self::Error> {
            let secp = Secp256k1::new();
            let key = self
                .wallet
                .derive_private_key(path)
                .map_err(|e| e.to_string())?;
            let message = Message::from_digest(sighash);
            Ok((
                secp.sign_ecdsa(&message, &key),
                PublicKey::from_secret_key(&secp, &key),
            ))
        }

        async fn public_key(&self, path: &DerivationPath) -> Result<PublicKey, Self::Error> {
            let secp = Secp256k1::new();
            let key = self
                .wallet
                .derive_private_key(path)
                .map_err(|e| e.to_string())?;
            Ok(PublicKey::from_secret_key(&secp, &key))
        }
    }

    /// A failed broadcast must release the UTXO reservation taken while building
    /// the transaction, so an immediate retry can reselect those inputs instead
    /// of failing with a spurious insufficient funds until the TTL backstop.
    #[tokio::test]
    async fn send_to_addresses_releases_reservation_on_broadcast_failure() {
        // Testnet wallet funded with a single spendable UTXO — the whole
        // balance rides on that one input, so a leaked reservation strands it.
        let (ctx, _funding_tx) = TestWalletContext::new_random()
            .with_mempool_funding(10_000_000)
            .await;
        let signer = WalletSigner {
            wallet: ctx.wallet.clone(),
        };

        let balance = Arc::new(WalletBalance::new());
        let info = PlatformWalletInfo {
            core_wallet: ctx.managed_wallet,
            balance: Arc::clone(&balance),
            identity_manager: IdentityManager::new(),
            tracked_asset_locks: BTreeMap::new(),
        };

        let mut wm = WalletManager::<PlatformWalletInfo>::new(Network::Testnet);
        let wallet_id = wm.insert_wallet(ctx.wallet, info).expect("insert wallet");
        let wallet_manager = Arc::new(RwLock::new(wm));

        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let broadcaster = Arc::new(FailFirstBroadcaster::new());
        let core = CoreWallet::new(
            sdk,
            Arc::clone(&wallet_manager),
            wallet_id,
            broadcaster,
            balance,
        );

        let recipient = DashAddress::dummy(Network::Testnet, 42);
        let outputs = vec![(recipient, 1_000_000u64)];

        // First attempt: build + sign succeed, broadcast fails.
        let first = core
            .send_to_addresses(
                StandardAccountType::BIP44Account,
                0,
                outputs.clone(),
                &signer,
            )
            .await;
        assert!(
            matches!(first, Err(PlatformWalletError::TransactionBroadcast(_))),
            "first send should surface the broadcast failure, got {first:?}"
        );

        // Immediate retry: only succeeds if the failed broadcast released the
        // reservation. With the leak, coin selection sees no spendable UTXO and
        // this fails with a build error instead.
        let second = core
            .send_to_addresses(StandardAccountType::BIP44Account, 0, outputs, &signer)
            .await;
        assert!(
            second.is_ok(),
            "retry after a failed broadcast should succeed once the reservation \
             is released, got {second:?}"
        );
    }
}
