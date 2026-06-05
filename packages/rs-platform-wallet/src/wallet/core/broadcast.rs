use dashcore::{Address as DashAddress, Transaction};
use key_wallet::account::account_type::StandardAccountType;
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::signer::Signer;

use crate::broadcaster::TransactionBroadcaster;
use crate::{CoreWallet, PlatformWalletError};

/// Max inputs per CoinJoin sweep transaction. A single Dash transaction must
/// stay under the standard relay/mempool size limit (`MAX_STANDARD_TX_SIZE` =
/// 100 000 B); at ~148 B/input (`INPUT_SIZE` below) that is ~675 inputs, so 500
/// leaves a comfortable margin for the output + overhead. A heavy mixer's UTXOs
/// are therefore swept across `ceil(N / MAX_INPUTS_PER_SWEEP)` transactions
/// rather than one oversized, unrelayable transaction.
const MAX_INPUTS_PER_SWEEP: usize = 500;

/// Balanced input count per sweep transaction for `total` spendable UTXOs, so
/// that `utxos.chunks(sweep_chunk_size(total))` yields `ceil(total /
/// MAX_INPUTS_PER_SWEEP)` near-equal chunks, each within `MAX_INPUTS_PER_SWEEP`.
///
/// Using a ceil-divided chunk size keeps chunks near-equal (e.g. 501 → 251 +
/// 250, not 500 + 1), so no chunk is a lone sub-fee dust input. `total` must be
/// greater than zero (the sweep early-returns on an empty UTXO set).
fn sweep_chunk_size(total: usize) -> usize {
    debug_assert!(total > 0, "sweep_chunk_size requires at least one UTXO");
    let num_chunks = total.div_ceil(MAX_INPUTS_PER_SWEEP);
    total.div_ceil(num_chunks)
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
    /// The UTXO set is split into balanced chunks of at most
    /// [`MAX_INPUTS_PER_SWEEP`] inputs so no transaction exceeds the standard
    /// relay size limit (a heavy mixer can hold thousands of small mixed-coin
    /// UTXOs). Each chunk spends a *disjoint* slice of the snapshot, so the
    /// transactions have no inter-dependency and may broadcast in any order.
    ///
    /// Within each chunk all inputs are added explicitly and the transaction
    /// is assembled and signed directly — it deliberately does NOT route
    /// through `TransactionBuilder::build_signed`, whose `LargestFirst` coin
    /// selection re-selects a *covering subset* and stops early, which can drop
    /// small UTXOs and leave the account non-empty. Each chunk's single output
    /// is `chunk_total - chunk_fee` (no change), so every UTXO is consumed.
    ///
    /// Returns the broadcast transactions in chunk order. Broadcast tolerates
    /// partial failure: the successfully broadcast transactions are returned
    /// (the caller refreshes balance and may re-run to sweep any remainder,
    /// since a re-run sees only the still-unspent UTXOs). An error is returned
    /// only if *no* transaction broadcast at all.
    pub async fn sweep_coinjoin_to_address<S: Signer>(
        &self,
        account_index: u32,
        dest: DashAddress,
        signer: &S,
    ) -> Result<Vec<Transaction>, PlatformWalletError> {
        use dashcore::blockdata::witness::Witness;
        use dashcore::{ScriptBuf, TxIn, TxOut};
        use key_wallet::wallet::managed_wallet_info::fee::FeeRate;
        use key_wallet::wallet::managed_wallet_info::transaction_builder::TransactionSigner;
        use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;

        // Build + sign every chunk under the wallet write lock (signing borrows
        // `managed_account` for address derivation), then broadcast after the
        // lock is released.
        let signed_txs: Vec<Transaction> = {
            let mut wm = self.wallet_manager.write().await;
            let (_wallet, info) = wm.get_wallet_and_info_mut(&self.wallet_id).ok_or_else(|| {
                PlatformWalletError::WalletNotFound(
                    "Wallet not found in wallet manager".to_string(),
                )
            })?;

            let current_height = info.core_wallet.synced_height();

            let managed_account = info
                .core_wallet
                .accounts
                .coinjoin_accounts
                .get(&account_index)
                .ok_or_else(|| {
                    PlatformWalletError::TransactionBuild(format!(
                        "CoinJoin managed account {} not found",
                        account_index
                    ))
                })?;

            // Snapshot every spendable UTXO — the sweep consumes all of them.
            let utxos: Vec<_> = managed_account
                .spendable_utxos(current_height)
                .into_iter()
                .cloned()
                .collect();

            if utxos.is_empty() {
                return Err(PlatformWalletError::TransactionBuild(
                    "no spendable CoinJoin UTXOs to sweep".to_string(),
                ));
            }

            let fee_rate = FeeRate::normal();
            const BASE_SIZE_1_OUTPUT_NO_CHANGE: usize = 8 + 1 + 1 + 34;
            const INPUT_SIZE: usize = 148;

            // Balanced chunks of <= MAX_INPUTS_PER_SWEEP so no transaction
            // exceeds the relay size limit. `chunks()` over disjoint slices
            // guarantees each UTXO is consumed by exactly one transaction.
            let chunk_size = sweep_chunk_size(utxos.len());
            let mut signed_txs = Vec::with_capacity(utxos.len().div_ceil(chunk_size));

            for chunk in utxos.chunks(chunk_size) {
                let chunk_utxos: Vec<_> = chunk.to_vec();
                let input_count = chunk_utxos.len();
                let total_input: u64 = chunk_utxos.iter().map(|u| u.value()).sum();

                // Exact fee for (input_count inputs, 1 output, no change).
                // Mirrors key-wallet's `calculate_base_size()` (8 + input-
                // varint + output-varint + 34) and the selector's 148 B/input,
                // so `total_input - fee` yields a single output with zero
                // change for this chunk.
                let fee = fee_rate
                    .calculate_fee(BASE_SIZE_1_OUTPUT_NO_CHANGE + input_count * INPUT_SIZE);

                if total_input <= fee {
                    return Err(PlatformWalletError::TransactionBuild(format!(
                        "CoinJoin sweep chunk balance {} is below the chunk fee {}",
                        total_input, fee
                    )));
                }
                let output_amount = total_input - fee;

                // Assemble the chunk's tx with ITS inputs explicitly and sign
                // it directly. Do NOT use `TransactionBuilder::build_signed` —
                // its `LargestFirst` coin selection re-selects a covering subset
                // and stops once `output_amount + fee` is met, which can drop
                // small UTXOs and leave the CoinJoin account non-empty. A sweep
                // must consume everything, so each chunk is an all-input,
                // single-output tx built by hand.
                let tx_inputs: Vec<TxIn> = chunk_utxos
                    .iter()
                    .map(|u| TxIn {
                        previous_output: u.outpoint,
                        script_sig: ScriptBuf::new(),
                        sequence: 0xffff_ffff, // Dash has no RBF
                        witness: Witness::new(),
                    })
                    .collect();
                let unsigned = Transaction {
                    version: 3,
                    lock_time: 0,
                    input: tx_inputs,
                    output: vec![TxOut {
                        value: output_amount,
                        script_pubkey: dest.script_pubkey(),
                    }],
                    special_transaction_payload: None,
                };

                // `sign_tx` signs `tx.input[i]` using `chunk_utxos[i]`, so input
                // order and the utxo vec must line up — both derive from `chunk`.
                let signed = signer
                    .sign_tx(unsigned, chunk_utxos, |addr| {
                        managed_account.address_derivation_path(&addr)
                    })
                    .await
                    .map_err(|e| PlatformWalletError::TransactionBuild(e.to_string()))?;

                debug_assert_eq!(
                    signed.input.len(),
                    input_count,
                    "CoinJoin sweep chunk must consume every UTXO in the chunk"
                );
                signed_txs.push(signed);
            }

            signed_txs
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
                Err(e) => last_err = Some(e),
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

#[cfg(test)]
mod sweep_chunking_tests {
    use super::{sweep_chunk_size, MAX_INPUTS_PER_SWEEP};

    /// The chunk plan must, for any UTXO count: produce `ceil(total / MAX)`
    /// transactions, keep every chunk within `MAX` inputs, and consume every
    /// UTXO exactly once (disjoint slices that sum back to `total`).
    #[test]
    fn partitions_every_utxo_within_the_relay_cap() {
        for &total in &[
            1usize, 30, 499, 500, 501, 675, 999, 1000, 1001, 1499, 5000, 12_345,
        ] {
            let chunk_size = sweep_chunk_size(total);
            let sizes: Vec<usize> = (0..total)
                .collect::<Vec<_>>()
                .chunks(chunk_size)
                .map(|c| c.len())
                .collect();

            let expected_chunks = total.div_ceil(MAX_INPUTS_PER_SWEEP);
            assert_eq!(sizes.len(), expected_chunks, "tx count for {total} UTXOs");
            assert_eq!(
                sizes.iter().sum::<usize>(),
                total,
                "every UTXO consumed exactly once for {total}"
            );
            assert!(
                sizes.iter().all(|&n| n >= 1 && n <= MAX_INPUTS_PER_SWEEP),
                "every chunk within [1, {MAX_INPUTS_PER_SWEEP}] for {total}: {sizes:?}"
            );
        }
    }
}
