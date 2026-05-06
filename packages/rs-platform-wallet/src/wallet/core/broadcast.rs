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

            // Peek at the next change address without advancing the derivation
            // index. We commit the advance only after post-build revalidation
            // succeeds, so a revalidation failure does not burn an index and
            // widen the gap-limit window on retry.
            let change_addr = change_account
                .next_change_address(Some(&xpub), false)
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

            // `select_inputs` is the only source of UTXOs for this builder,
            // so `tx.input` outpoints must be a subset of the height-aware
            // `spendable` set by the builder's contract. The check below is
            // a defense-in-depth runtime guard for builder regressions;
            // under normal operation this branch is unreachable. Inputs are
            // not marked spent here either way — that happens after a
            // successful broadcast (see #3466 review): a failed broadcast
            // must not leave UTXOs falsely marked spent.
            let selected: BTreeSet<OutPoint> =
                tx.input.iter().map(|txin| txin.previous_output).collect();
            let spendable_outpoints: BTreeSet<OutPoint> =
                spendable.iter().map(|utxo| utxo.outpoint).collect();
            if !selected.is_subset(&spendable_outpoints) {
                // INTENTIONAL(CMT-002): The `ConcurrentSpendConflict` variant
                // is named and framed as user-retryable for forward
                // compatibility. The current code path is only reachable on
                // a builder-internal regression, but the typed variant is
                // preserved so future work that surfaces real concurrent-spend
                // conflicts (e.g. from cross-process wallets) can route
                // through the same handler without an API churn.
                return Err(PlatformWalletError::ConcurrentSpendConflict);
            }

            // Revalidation passed; now commit the change-address advance so
            // the next send picks up the next index. Re-borrow the managed
            // account because `select_inputs` above borrowed
            // `info.core_wallet.accounts` and ended the earlier reborrow.
            let change_account = match account_type {
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
            .ok_or_else(|| {
                PlatformWalletError::TransactionBuild(format!(
                    "{:?} managed account {} not found",
                    account_type, account_index
                ))
            })?;
            change_account
                .next_change_address(Some(&xpub), true)
                .map_err(|e| PlatformWalletError::TransactionBuild(e.to_string()))?;

            tx
        };

        // Broadcast first; if the network rejects we leave wallet state
        // untouched so the caller can retry without manual sync repair.
        // This is intentional even if the remote accepted the transaction
        // but the broadcast path returned an error: in that ambiguous case
        // later attempts may reuse the same inputs locally, but the network
        // rejects the duplicate spend instead of us marking UTXOs spent for
        // a transaction that might not have propagated.
        self.broadcast_transaction(&tx).await?;

        // Now that the tx is in flight, register it as a mempool transaction
        // so subsequent callers see the inputs as spent and don't reselect
        // them. The trade-off is that two callers racing between the lock
        // drop above and the broadcast can both pick the same UTXOs; the
        // network resolves that race exactly as it does on `v3.1-dev`
        // today, but neither caller corrupts local state on a transient
        // broadcast failure.
        //
        // Broadcast-first semantics: by the time we get here the network has
        // already accepted the transaction, so the two warning paths below
        // intentionally do NOT convert into a post-success `Err`. They
        // simply mean local wallet state did not get updated to reflect the
        // mempool spend / change output. Recovery in both cases:
        //
        //   * The next `send_to_addresses` from the same handle may reselect
        //     the same UTXOs because they still look spendable locally. That
        //     follow-up transaction will be rejected by the network as a
        //     duplicate spend (the broadcaster surfaces that as an error to
        //     the caller), so funds are never double-spent on-chain.
        //   * Once mempool/block sync catches up, the wallet will see the
        //     original transaction and reconcile its UTXO set, after which
        //     subsequent sends pick up the correct change outputs.
        //
        // The two cases differ in what they imply:
        //
        //   * `!check_result.is_relevant` is the expected transient: the
        //     wallet just hasn't ingested the tx yet (or some derivation
        //     path/script is unrecognised), and a later sync will fix it.
        //   * The `else` branch (wallet missing in the manager) is NOT a
        //     normal transient — the broadcast succeeded against a
        //     `CoreWallet` handle whose underlying wallet entry is gone
        //     from the manager. That is a broken/inconsistent local handle
        //     and the warning exists so operators can spot it; future
        //     sends through the same handle will keep failing the lookup
        //     above and surface a clean `WalletNotFound` error.
        {
            let mut wm = self.wallet_manager.write().await;
            if let Some((wallet, info)) = wm.get_wallet_mut_and_info_mut(&self.wallet_id) {
                let check_result = info
                    .check_core_transaction(&tx, TransactionContext::Mempool, wallet, true, true)
                    .await;
                if !check_result.is_relevant {
                    // CMT-004: The wallet just built and signed this
                    // transaction from its own spendable inputs, so a
                    // `!is_relevant` post-broadcast check is an
                    // internal-invariant violation, not a transient. Emit a
                    // structured `error!` event with stable field names so
                    // operators can alert on it independent of the message
                    // text. We still return `Ok(tx)`: broadcast already
                    // succeeded, and rolling back here would mislead the
                    // caller into thinking the network rejected the tx.
                    tracing::error!(
                        target: "platform_wallet::broadcast",
                        event = "post_broadcast_unrelated_to_own_wallet",
                        txid = %tx.txid(),
                        wallet_id = %hex::encode(self.wallet_id),
                        "Internal invariant violation: own-built broadcast not recognized by post-broadcast check"
                    );
                }
            } else {
                // INTENTIONAL(CMT-005): The wallet-missing branch indicates
                // the wallet entry was removed from the manager between the
                // lock drop and re-acquisition. Broadcast already succeeded,
                // so converting to `Err` would be wrong (caller would think
                // the tx failed). Observability via a single structured log
                // line is acceptable for current operator workflows —
                // promote to a metric only when monitoring infrastructure is
                // in place to consume one.
                tracing::warn!(
                    wallet_id = %hex::encode(self.wallet_id),
                    txid = %tx.txid(),
                    "wallet missing during post-broadcast transaction registration"
                );
            }
        }

        Ok(tx)
    }
}
