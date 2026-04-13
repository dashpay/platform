//! Simple payment transaction building, signing, and broadcasting.

use dashcore::secp256k1::{Message, Secp256k1};
use dashcore::sighash::SighashCache;
use dashcore::Address as DashAddress;
use dashcore::{OutPoint, ScriptBuf, Transaction, TxIn, TxOut};
use key_wallet::Utxo;

use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;

use crate::error::PlatformWalletError;

use super::CoreWallet;

/// Minimum fee for a transaction (duffs).
const MIN_ASSET_LOCK_FEE: u64 = 3_000;

/// Minimum value for a change output (duffs). Outputs below this threshold are
/// considered dust and will be rejected by the network.
const DUST_THRESHOLD: u64 = 546;

/// Estimate the transaction size in bytes for a standard (non-special) transaction.
///
/// Assumes P2PKH inputs (~148 B each), standard outputs (~34 B each),
/// and a ~10 B header.
fn estimate_standard_tx_size(num_inputs: usize, num_outputs: usize) -> usize {
    10 + (num_inputs * 148) + (num_outputs * 34)
}

impl CoreWallet {
    /// Build, sign, and broadcast a simple payment transaction.
    ///
    /// Creates a standard P2PKH transaction sending the specified amounts to
    /// the given addresses. The method performs the following steps:
    ///
    /// 1. Collects spendable UTXOs from the wallet.
    /// 2. Selects UTXOs covering the total output value plus an estimated fee.
    /// 3. Builds the transaction with the requested outputs and a change
    ///    output (if above dust threshold).
    /// 4. Signs all inputs using the private keys derived from the wallet.
    /// 5. Broadcasts the transaction via the injected [`TransactionBroadcaster`].
    ///
    /// Returns the signed and broadcast transaction.
    pub async fn send_transaction(
        &self,
        outputs: Vec<(DashAddress, u64)>,
    ) -> Result<Transaction, PlatformWalletError> {
        if outputs.is_empty() {
            return Err(PlatformWalletError::TransactionBuild(
                "No outputs specified".to_string(),
            ));
        }

        let total_output: u64 = outputs
            .iter()
            .try_fold(0u64, |acc, (_, amount)| acc.checked_add(*amount))
            .ok_or_else(|| {
                PlatformWalletError::TransactionBuild("Output amount overflow".into())
            })?;
        if total_output == 0 {
            return Err(PlatformWalletError::TransactionBuild(
                "Total output amount must be greater than zero".to_string(),
            ));
        }

        let secp = Secp256k1::new();

        // 1. Get spendable UTXOs.
        // TODO: Not a good idea to clone all spendable utoxs. It should be better way to create transactions. should we use dashcore transation builder? this seems like a logic that belongs to dashcore
        let spendable: Vec<Utxo> = {
            let wm = self.wallet_manager.read().await;
            let info = wm.get_wallet_info(&self.wallet_id).expect("wallet exists");
            info.get_spendable_utxos().into_iter().cloned().collect()
        };

        if spendable.is_empty() {
            return Err(PlatformWalletError::TransactionBuild(
                "No spendable UTXOs available".to_string(),
            ));
        }

        // 2. Select UTXOs using greedy largest-first strategy.
        let (selected_utxos, fee, change) =
            self.select_utxos_for_payment(&spendable, total_output, outputs.len())?;

        // 3. Build the transaction outputs.
        let mut tx_outputs: Vec<TxOut> = outputs
            .iter()
            .map(|(addr, amount)| TxOut {
                value: *amount,
                script_pubkey: addr.script_pubkey(),
            })
            .collect();

        let _ = fee; // fee is consumed implicitly (inputs - outputs - change)

        if let Some(change_value) = change {
            let change_addr = self.next_change_address().await?;
            tx_outputs.push(TxOut {
                value: change_value,
                script_pubkey: change_addr.script_pubkey(),
            });
        }

        // 4. Build inputs.
        let inputs: Vec<TxIn> = selected_utxos
            .iter()
            .map(|(outpoint, _, _)| TxIn {
                previous_output: *outpoint,
                ..Default::default()
            })
            .collect();

        let mut tx = Transaction {
            version: 2,
            lock_time: 0,
            input: inputs,
            output: tx_outputs,
            special_transaction_payload: None,
        };

        // 5. Sign all inputs.
        self.sign_transaction_inputs(&secp, &mut tx, &selected_utxos)
            .await?;

        // 6. Broadcast.
        self.broadcast_transaction(&tx).await?;

        Ok(tx)
    }

    /// Select UTXOs covering `total_output + fee` for a standard payment.
    ///
    /// Uses a greedy largest-first strategy. Returns the selected UTXOs,
    /// the fee in duffs, and an optional change value.
    fn select_utxos_for_payment(
        &self,
        spendable: &[Utxo],
        total_output: u64,
        num_payment_outputs: usize,
    ) -> Result<(Vec<(OutPoint, TxOut, DashAddress)>, u64, Option<u64>), PlatformWalletError> {
        let mut sorted: Vec<&Utxo> = spendable.iter().collect();
        sorted.sort_by(|a, b| b.value().cmp(&a.value()));

        // Iterative fee estimation: start with a rough estimate and refine.
        let mut fee_estimate = std::cmp::max(
            MIN_ASSET_LOCK_FEE,
            estimate_standard_tx_size(1, num_payment_outputs + 1) as u64,
        );

        for _ in 0..2 {
            let target = total_output.saturating_add(fee_estimate);

            let mut selected = Vec::new();
            let mut total_input = 0u64;

            for utxo in &sorted {
                if total_input >= target {
                    break;
                }
                selected.push((utxo.outpoint, utxo.txout.clone(), utxo.address.clone()));
                total_input += utxo.value();
            }

            if total_input < total_output.saturating_add(MIN_ASSET_LOCK_FEE) {
                return Err(PlatformWalletError::TransactionBuild(format!(
                    "Insufficient funds: need {} + fee, have {}",
                    total_output, total_input
                )));
            }

            // Recompute fee based on actual input count.
            let fee_with_change = std::cmp::max(
                MIN_ASSET_LOCK_FEE,
                estimate_standard_tx_size(selected.len(), num_payment_outputs + 1) as u64,
            );
            let tentative_change = total_input
                .checked_sub(total_output)
                .and_then(|r| r.checked_sub(fee_with_change));

            if let Some(change) = tentative_change {
                if change >= DUST_THRESHOLD {
                    return Ok((selected, fee_with_change, Some(change)));
                }
            }

            // No change (or dust): recompute fee without change output.
            let fee_no_change = std::cmp::max(
                MIN_ASSET_LOCK_FEE,
                estimate_standard_tx_size(selected.len(), num_payment_outputs) as u64,
            );

            if total_input >= total_output.saturating_add(fee_no_change) {
                let actual_fee = total_input - total_output;
                return Ok((selected, actual_fee, None));
            }

            fee_estimate = fee_with_change;
        }

        Err(PlatformWalletError::TransactionBuild(format!(
            "Insufficient funds after retry: need {} + fee {}",
            total_output, fee_estimate
        )))
    }

    // TODO: Don't we have it dashcore?
    /// Sign all inputs of a transaction using P2PKH.
    ///
    /// For each input, looks up the UTXO address, finds the corresponding
    /// derivation path in the wallet info, derives the private key, and
    /// constructs the scriptSig.
    pub(crate) async fn sign_transaction_inputs(
        &self,
        secp: &Secp256k1<dashcore::secp256k1::All>,
        tx: &mut Transaction,
        selected_utxos: &[(OutPoint, TxOut, DashAddress)],
    ) -> Result<(), PlatformWalletError> {
        let sighash_u32 = 1u32; // SIGHASH_ALL

        // Compute sighashes first (immutable borrow of tx).
        let cache = SighashCache::new(&*tx);
        let sighashes: Vec<_> = tx
            .input
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let (_, txout, _) = &selected_utxos[i];
                cache
                    .legacy_signature_hash(i, &txout.script_pubkey, sighash_u32)
                    .map_err(|e| {
                        PlatformWalletError::TransactionBuild(format!(
                            "Failed to compute sighash for input {}: {}",
                            i, e
                        ))
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        drop(cache);

        // Look up derivation paths and derive private keys under a single lock.
        let wm = self.wallet_manager.read().await;
        let (wallet, info) = wm
            .get_wallet_and_info(&self.wallet_id)
            .expect("wallet exists");
        let derivation_paths = selected_utxos
            .iter()
            .map(|(_, _, address)| {
                for account in info.core_wallet.accounts.all_accounts() {
                    if let Some(path) = account.address_derivation_path(address) {
                        return Ok(path);
                    }
                }
                Err(PlatformWalletError::TransactionBuild(format!(
                    "Address {} not found in wallet",
                    address
                )))
            })
            .collect::<Result<Vec<_>, _>>()?;

        // Derive private keys and sign.
        for (i, (input, sighash)) in tx.input.iter_mut().zip(sighashes).enumerate() {
            let path = &derivation_paths[i];
            let extended_key = wallet.derive_extended_private_key(path).map_err(|e| {
                PlatformWalletError::TransactionBuild(format!(
                    "Failed to derive key for input {}: {}",
                    i, e
                ))
            })?;
            let input_private_key = extended_key.to_priv();

            let message = Message::from_digest(sighash.into());
            let sig = secp.sign_ecdsa(&message, &input_private_key.inner);

            // Build scriptSig: <sig_len+1> <der_sig> <sighash_byte> <pubkey_len> <pubkey>
            let mut der_sig = sig.serialize_der().to_vec();
            let mut script_sig = vec![(der_sig.len() + 1) as u8];
            script_sig.append(&mut der_sig);
            script_sig.push(1u8); // SIGHASH_ALL

            let pub_key_bytes = input_private_key.public_key(secp).inner.serialize();
            script_sig.push(pub_key_bytes.len() as u8);
            script_sig.extend_from_slice(&pub_key_bytes);

            input.script_sig = ScriptBuf::from_bytes(script_sig);
        }

        Ok(())
    }
}
