use dashcore::{Address as DashAddress, Transaction};
use key_wallet::account::account_type::StandardAccountType;
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::signer::Signer;

use crate::broadcaster::TransactionBroadcaster;
use crate::{CoreWallet, PlatformWalletError};

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

        self.broadcast_transaction(&tx).await?;
        Ok(tx)
    }

    /// Sweep the *entire* spendable balance of a CoinJoin account to `dest`,
    /// leaving no change behind, across one or more transactions.
    ///
    /// CoinJoin "mixed coins" live on a dedicated CoinJoin account (BIP44
    /// purpose 4'), which [`send_to_addresses`](Self::send_to_addresses)
    /// cannot reach — it only resolves standard BIP44/BIP32 accounts. This
    /// is used by the DashSync → SwiftDashSDK migration to move a user's
    /// mixed coins (no longer supported) into their spendable balance.
    ///
    /// The chunking, dual-chain (`/0/` + `/1/`) signing-path resolution, and
    /// all-input/no-change transaction building live upstream in key-wallet
    /// ([`ManagedCoreFundsAccount::build_coinjoin_sweep_txs`](key_wallet::managed_account::ManagedCoreFundsAccount::build_coinjoin_sweep_txs)).
    /// This wrapper only resolves the account under the wallet lock, delegates
    /// the build+sign, then broadcasts.
    ///
    /// Broadcast tolerates partial failure: the successfully broadcast
    /// transactions are returned (the caller refreshes balance and may re-run
    /// to sweep any remainder, since a re-run sees only the still-unspent
    /// UTXOs). An error is returned only if *no* transaction broadcast at all.
    pub async fn sweep_coinjoin_to_address<S: Signer>(
        &self,
        account_index: u32,
        dest: DashAddress,
        signer: &S,
    ) -> Result<Vec<Transaction>, PlatformWalletError> {
        use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;

        // Build + sign every chunk under the wallet write lock (signing borrows
        // the managed account for address derivation), then broadcast after the
        // lock is released.
        let signed_txs: Vec<Transaction> = {
            let mut wm = self.wallet_manager.write().await;
            let (wallet, info) = wm.get_wallet_and_info_mut(&self.wallet_id).ok_or_else(|| {
                PlatformWalletError::WalletNotFound(
                    "Wallet not found in wallet manager".to_string(),
                )
            })?;

            // The CoinJoin account's watch-only public xpub. The managed account
            // doesn't store it, so it's read from the wallet side and passed to
            // the upstream builder to re-derive signing paths across both chains
            // (no private key crosses any boundary). `Copy`, so the immutable
            // `wallet` borrow ends here, before the `info` borrow below.
            let account_xpub = wallet
                .accounts
                .coinjoin_accounts
                .get(&account_index)
                .ok_or_else(|| {
                    PlatformWalletError::WalletNotFound(format!(
                        "CoinJoin account {account_index} not found"
                    ))
                })?
                .account_xpub;

            let current_height = info.core_wallet.synced_height();
            let managed_account = info
                .core_wallet
                .accounts
                .coinjoin_accounts
                .get(&account_index)
                .ok_or_else(|| {
                    PlatformWalletError::TransactionBuild(format!(
                        "CoinJoin managed account {account_index} not found"
                    ))
                })?;

            managed_account
                .build_coinjoin_sweep_txs(account_xpub, current_height, dest, signer)
                .await
                .map_err(|e| PlatformWalletError::TransactionBuild(e.to_string()))?
        };

        // Broadcast each chunk (disjoint inputs, no inter-tx dependency, so
        // order is irrelevant). Collect successes and tolerate partial failure
        // so a flaky broadcast doesn't strand the chunks that did go out — the
        // caller can re-run to sweep any remainder. Error only if nothing
        // broadcast at all.
        let mut broadcast: Vec<Transaction> = Vec::with_capacity(signed_txs.len());
        let mut last_err: Option<PlatformWalletError> = None;
        for tx in signed_txs {
            match self.broadcast_transaction(&tx).await {
                Ok(_) => broadcast.push(tx),
                Err(e) => {
                    // Partial failure is tolerated (caller re-runs to sweep the
                    // remainder), but never silent: log each dropped chunk error.
                    tracing::warn!(
                        "CoinJoin sweep: a chunk failed to broadcast, continuing \
                         with remaining chunks (caller can re-run): {}",
                        e
                    );
                    // Keep the FIRST failure (usually the root cause); the later
                    // chunk errors are already surfaced via the warn! above.
                    last_err.get_or_insert(e);
                }
            }
        }

        if broadcast.is_empty() {
            return Err(last_err.unwrap_or_else(|| {
                PlatformWalletError::TransactionBuild(
                    "CoinJoin sweep produced no broadcastable transactions".to_string(),
                )
            }));
        }

        Ok(broadcast)
    }
}
