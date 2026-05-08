use std::collections::BTreeSet;

use dashcore::{Address as DashAddress, OutPoint, Transaction};
use key_wallet::account::account_type::StandardAccountType;
use key_wallet::transaction_checking::{TransactionContext, WalletTransactionChecker};
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;

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
    /// Uses key-wallet's [`TransactionBuilder`] for UTXO selection, fee
    /// estimation, and signing. Change is sent to the next internal address
    /// of the specified account.
    pub async fn send_to_addresses(
        &self,
        account_type: StandardAccountType,
        account_index: u32,
        outputs: Vec<(DashAddress, u64)>,
    ) -> Result<Transaction, PlatformWalletError> {
        use key_wallet::wallet::managed_wallet_info::coin_selection::SelectionStrategy;
        use key_wallet::wallet::managed_wallet_info::transaction_builder::TransactionBuilder;

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

            // Look up managed account and immutable Account (for xpub) based on type.
            let (managed_accounts, wallet_accounts) = match account_type {
                StandardAccountType::BIP44Account => (
                    &mut info.core_wallet.accounts.standard_bip44_accounts,
                    &wallet.accounts.standard_bip44_accounts,
                ),
                StandardAccountType::BIP32Account => (
                    &mut info.core_wallet.accounts.standard_bip32_accounts,
                    &wallet.accounts.standard_bip32_accounts,
                ),
            };

            let account = managed_accounts.get(&account_index).ok_or_else(|| {
                PlatformWalletError::TransactionBuild(format!(
                    "{:?} account {} not found",
                    account_type, account_index
                ))
            })?;

            let spendable: Vec<_> = account
                .spendable_utxos(current_height)
                .into_iter()
                .cloned()
                .collect();

            let xpub = wallet_accounts
                .get(&account_index)
                .map(|a| a.account_xpub)
                .ok_or_else(|| {
                    PlatformWalletError::TransactionBuild(format!(
                        "{:?} account {} not found in wallet",
                        account_type, account_index
                    ))
                })?;

            let mut builder = TransactionBuilder::new();
            for (addr, amount) in &outputs {
                builder = builder
                    .add_output(addr, *amount)
                    .map_err(|e| PlatformWalletError::TransactionBuild(e.to_string()))?;
            }

            // Need mutable access for change address derivation.
            let change_account = managed_accounts.get_mut(&account_index).ok_or_else(|| {
                PlatformWalletError::TransactionBuild(format!(
                    "{:?} managed account {} not found",
                    account_type, account_index
                ))
            })?;

            let change_addr = change_account
                .next_change_address(Some(&xpub), true)
                .map_err(|e| PlatformWalletError::TransactionBuild(e.to_string()))?;

            builder = builder.set_change_address(change_addr);

            builder = builder
                .select_inputs(
                    &spendable,
                    SelectionStrategy::LargestFirst,
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
                    },
                )
                .map_err(|e| PlatformWalletError::TransactionBuild(e.to_string()))?;

            let tx = builder
                .build()
                .map_err(|e| PlatformWalletError::TransactionBuild(e.to_string()))?;

            // Re-validate the selected outpoints are still spendable while
            // we still hold the write lock. The lock makes our build atomic
            // against other callers on this handle, but external mempool /
            // block events processed before we acquired the lock may have
            // invalidated UTXOs that were still in the spendable set when
            // `select_inputs` ran.
            //
            // We deliberately do NOT mark the inputs as spent here — that
            // happens after a successful broadcast (see #3466 review). A
            // failed broadcast must not leave UTXOs falsely marked spent.
            let selected: BTreeSet<OutPoint> =
                tx.input.iter().map(|txin| txin.previous_output).collect();
            let still_spendable: BTreeSet<OutPoint> = info
                .get_spendable_utxos()
                .into_iter()
                .map(|utxo| utxo.outpoint)
                .collect();
            if !selected.is_subset(&still_spendable) {
                return Err(PlatformWalletError::TransactionBuild(
                    "Selected UTXOs are no longer available (concurrent transaction). \
                     Please retry."
                        .to_string(),
                ));
            }

            tx
        };

        // Broadcast first; if the network rejects we leave wallet state
        // untouched so the caller can retry without manual sync repair.
        self.broadcast_transaction(&tx).await?;

        // Now that the tx is in flight, register it as a mempool transaction
        // so subsequent callers see the inputs as spent and don't reselect
        // them. The trade-off is that two callers racing between the lock
        // drop above and the broadcast can both pick the same UTXOs; the
        // network resolves that race exactly as it does on `v3.1-dev`
        // today, but neither caller corrupts local state on a transient
        // broadcast failure.
        // Post-broadcast hook must mark consumed UTXOs spent on every
        // standard-tx account collection (BIP44 + BIP32). Pinned by
        // `cr_004_legacy_bip32_utxo_update_after_spend` (dash-evo-tool#845).
        {
            let mut wm = self.wallet_manager.write().await;
            let (wallet, info) =
                wm.get_wallet_mut_and_info_mut(&self.wallet_id)
                    .ok_or_else(|| {
                        crate::error::PlatformWalletError::WalletNotFound(
                            "Wallet not found in wallet manager".to_string(),
                        )
                    })?;
            info.check_core_transaction(&tx, TransactionContext::Mempool, wallet, true, true)
                .await;
        }

        Ok(tx)
    }
}
