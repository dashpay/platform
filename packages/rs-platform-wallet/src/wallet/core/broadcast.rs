use dashcore::{Address as DashAddress, Transaction};

use crate::{CoreWallet, PlatformWalletError};

impl CoreWallet {
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
    /// Uses key-wallet's [`TransactionBuilder`] for UTXO selection, fee
    /// estimation, and signing. Change is sent to the next internal address
    /// of BIP-44 account 0.
    pub async fn send_to_addresses(
        &self,
        outputs: Vec<(DashAddress, u64)>,
    ) -> Result<Transaction, PlatformWalletError> {
        use key_wallet::wallet::managed_wallet_info::transaction_builder::TransactionBuilder;
        use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;

        if outputs.is_empty() {
            return Err(PlatformWalletError::TransactionBuild(
                "No outputs specified".to_string(),
            ));
        }

        let tx = {
            let mut wm = self.wallet_manager.write().await;
            let (wallet, info) = wm
                .get_wallet_and_info_mut(&self.wallet_id)
                .expect("wallet exists");

            let current_height = info.core_wallet.synced_height();
            let account = info
                .core_wallet
                .accounts
                .standard_bip44_accounts
                .get(&0)
                .ok_or_else(|| {
                    PlatformWalletError::TransactionBuild("BIP-44 account 0 not found".to_string())
                })?;
            let spendable: Vec<_> = account
                .spendable_utxos(current_height)
                .into_iter()
                .cloned()
                .collect();

            let mut builder = TransactionBuilder::new();
            for (addr, amount) in &outputs {
                builder = builder
                    .add_output(addr, *amount)
                    .map_err(|e| PlatformWalletError::TransactionBuild(e.to_string()))?;
            }

            let change_xpub = wallet
                .accounts
                .standard_bip44_accounts
                .get(&0)
                .map(|a| a.account_xpub)
                .ok_or_else(|| {
                    PlatformWalletError::TransactionBuild("BIP-44 account 0 not found".to_string())
                })?;

            let change_account = info
                .core_wallet
                .accounts
                .standard_bip44_accounts
                .get_mut(&0)
                .ok_or_else(|| {
                    PlatformWalletError::TransactionBuild(
                        "BIP-44 managed account 0 not found".to_string(),
                    )
                })?;

            let change_addr = change_account
                .next_change_address(Some(&change_xpub))
                .map_err(|e| PlatformWalletError::TransactionBuild(e.to_string()))?;

            builder = builder.set_change_address(change_addr);

            builder = builder
                .select_inputs(
                    &spendable,
                    key_wallet::wallet::managed_wallet_info::coin_selection::SelectionStrategy::LargestFirst,
                    current_height,
                    |utxo| {
                    for account in info.core_wallet.accounts.all_accounts() {
                        if let Some(path) = account.address_derivation_path(&utxo.address) {
                            if let Ok(key) = wallet.derive_private_key(&path) {
                                return Some(key);
                            }
                        }
                    }
                    None
                })
                .map_err(|e| PlatformWalletError::TransactionBuild(e.to_string()))?;

            builder
                .build()
                .map_err(|e| PlatformWalletError::TransactionBuild(e.to_string()))?
        };

        self.broadcast_transaction(&tx).await?;
        Ok(tx)
    }
}
