//! Core wallet functionality: balance, UTXOs, addresses, transaction history.

use std::sync::Arc;

use super::balance::WalletBalance;

use dashcore::consensus;
use dashcore::secp256k1::{Message, Secp256k1};
use dashcore::sighash::SighashCache;
use dashcore::Address as DashAddress;
use dashcore::{OutPoint, PrivateKey, ScriptBuf, Transaction, TxIn, TxOut};
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use key_wallet::Utxo;
use tokio::sync::RwLock;

use crate::error::PlatformWalletError;


/// Write guard for `ManagedWalletInfo` that automatically refreshes
/// `WalletBalance` when dropped. Ensures the lock-free balance is always
/// consistent with the wallet info after any mutation.
pub struct WalletInfoWriteGuard<'a> {
    guard: tokio::sync::RwLockWriteGuard<'a, ManagedWalletInfo>,
    balance: &'a WalletBalance,
}

impl<'a> std::ops::Deref for WalletInfoWriteGuard<'a> {
    type Target = ManagedWalletInfo;
    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

impl std::ops::DerefMut for WalletInfoWriteGuard<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.guard
    }
}

impl Drop for WalletInfoWriteGuard<'_> {
    fn drop(&mut self) {
        self.balance.update(&self.guard.balance());
    }
}

/// Core wallet providing UTXO, balance, and address functionality.
#[derive(Clone)]
pub struct CoreWallet {
    pub(crate) sdk: Arc<dash_sdk::Sdk>,
    pub(crate) wallet: Arc<RwLock<Wallet>>,
    /// Private — always access through `wallet_info()`, `wallet_info_mut()`,
    /// `try_wallet_info()`, or `try_wallet_info_mut()`. Write access returns
    /// `WalletInfoWriteGuard` which auto-refreshes `WalletBalance` on drop.
    wallet_info: Arc<RwLock<ManagedWalletInfo>>,
    /// Lock-free balance — updated from `ManagedWalletInfo` on every
    /// SPV block/mempool processing and RPC refresh. Read without any lock.
    pub(crate) balance: WalletBalance,
}

impl CoreWallet {
    /// Create a new CoreWallet.
    pub(crate) fn new(
        sdk: Arc<dash_sdk::Sdk>,
        wallet: Arc<RwLock<Wallet>>,
        wallet_info: Arc<RwLock<ManagedWalletInfo>>,
    ) -> Self {
        Self {
            sdk,
            wallet,
            wallet_info,
            balance: WalletBalance::new(),
        }
    }

    /// Lock-free balance — read from any context without locking.
    /// Updated automatically after SPV/RPC balance changes.
    pub fn balance(&self) -> &WalletBalance {
        &self.balance
    }

    /// Read access to the underlying `ManagedWalletInfo`.
    ///
    /// Use this when you need multiple reads in a single lock acquisition
    /// (balance + UTXOs + addresses, etc.) to avoid redundant locking.
    pub async fn wallet_info(&self) -> tokio::sync::RwLockReadGuard<'_, ManagedWalletInfo> {
        self.wallet_info.read().await
    }

    /// Write access to the underlying `ManagedWalletInfo`.
    ///
    /// Returns a guard that automatically refreshes `WalletBalance` when dropped,
    /// so the lock-free balance is always consistent with `ManagedWalletInfo`.
    pub async fn wallet_info_mut(&self) -> WalletInfoWriteGuard<'_> {
        let guard = self.wallet_info.write().await;
        WalletInfoWriteGuard {
            guard,
            balance: &self.balance,
        }
    }

    /// Blocking read access to the underlying `ManagedWalletInfo`.
    ///
    /// Blocks the current thread until the read lock is acquired.
    /// Use from synchronous contexts (e.g. egui UI) where awaiting is
    /// not possible. Equivalent to `std::sync::RwLock::read()`.
    ///
    /// # Panics
    ///
    /// Panics if called from an async context (use `wallet_info().await`
    /// instead).
    pub fn wallet_info_blocking(&self) -> tokio::sync::RwLockReadGuard<'_, ManagedWalletInfo> {
        self.wallet_info.blocking_read()
    }

    /// Non-blocking read access to the underlying `ManagedWalletInfo`.
    ///
    /// Returns `None` if a writer currently holds the lock. Useful in
    /// synchronous contexts (e.g. `spawn_blocking`) where awaiting is not
    /// possible.
    pub fn try_wallet_info(&self) -> Option<tokio::sync::RwLockReadGuard<'_, ManagedWalletInfo>> {
        self.wallet_info.try_read().ok()
    }

    /// Non-blocking write access to the underlying `ManagedWalletInfo`.
    ///
    /// Returns `None` if the lock is currently held. Useful in synchronous
    /// contexts (e.g. `spawn_blocking`) where awaiting is not possible.
    pub fn try_wallet_info_mut(&self) -> Option<WalletInfoWriteGuard<'_>> {
        self.wallet_info
            .try_write()
            .ok()
            .map(|guard| WalletInfoWriteGuard {
                guard,
                balance: &self.balance,
            })
    }

    /// Read access to the underlying `Wallet` (key material).
    pub async fn wallet(&self) -> tokio::sync::RwLockReadGuard<'_, Wallet> {
        self.wallet.read().await
    }

    /// Blocking read access to the underlying `Wallet` (key material).
    ///
    /// # Panics
    /// Panics if called from an async context (use `wallet().await` instead).
    pub fn wallet_blocking(&self) -> tokio::sync::RwLockReadGuard<'_, Wallet> {
        self.wallet.blocking_read()
    }

    /// Blocking write access to the underlying `Wallet` (key material).
    ///
    /// # Panics
    /// Panics if called from an async context (use `wallet().write().await` instead).
    pub fn wallet_mut_blocking(&self) -> tokio::sync::RwLockWriteGuard<'_, Wallet> {
        self.wallet.blocking_write()
    }

    /// Get the next unused receive address for the default account.
    pub async fn next_receive_address(
        &self,
    ) -> Result<DashAddress, crate::error::PlatformWalletError> {
        self.next_receive_address_for_account(0).await
    }

    /// Get the next unused BIP-44 external (receive) address for a specific account.
    pub async fn next_receive_address_for_account(
        &self,
        account_index: u32,
    ) -> Result<DashAddress, crate::error::PlatformWalletError> {
        let xpub = self.derive_account_xpub(account_index).await?;
        let mut info = self.wallet_info.write().await;
        let account = info
            .accounts
            .standard_bip44_accounts
            .get_mut(&account_index)
            .ok_or_else(|| {
                crate::error::PlatformWalletError::WalletCreation(format!(
                    "BIP-44 account {} not found",
                    account_index
                ))
            })?;
        account
            .next_receive_address(Some(&xpub), true)
            .map_err(|e| crate::error::PlatformWalletError::WalletCreation(e.to_string()))
    }

    /// Blocking version of `next_receive_address` for sync contexts.
    pub fn next_receive_address_blocking(
        &self,
    ) -> Result<DashAddress, crate::error::PlatformWalletError> {
        self.next_receive_address_for_account_blocking(0)
    }

    /// Blocking version of `next_receive_address_for_account`.
    pub fn next_receive_address_for_account_blocking(
        &self,
        account_index: u32,
    ) -> Result<DashAddress, crate::error::PlatformWalletError> {
        let xpub = {
            let wallet = self.wallet.blocking_read();
            let path = key_wallet::account::AccountType::Standard {
                index: account_index,
                standard_account_type: key_wallet::account::StandardAccountType::BIP44Account,
            }
            .derivation_path(wallet.network)
            .map_err(|e| crate::error::PlatformWalletError::WalletCreation(e.to_string()))?;
            wallet
                .derive_extended_public_key(&path)
                .map_err(|e| crate::error::PlatformWalletError::WalletCreation(e.to_string()))?
        };
        let mut info = self.wallet_info.blocking_write();
        let account = info
            .accounts
            .standard_bip44_accounts
            .get_mut(&account_index)
            .ok_or_else(|| {
                crate::error::PlatformWalletError::WalletCreation(format!(
                    "BIP-44 account {} not found",
                    account_index
                ))
            })?;
        account
            .next_receive_address(Some(&xpub), true)
            .map_err(|e| crate::error::PlatformWalletError::WalletCreation(e.to_string()))
    }

    /// Get the next unused change address for the default account.
    pub async fn next_change_address(
        &self,
    ) -> Result<DashAddress, crate::error::PlatformWalletError> {
        self.next_change_address_for_account(0).await
    }

    /// Blocking version of `next_change_address` for sync contexts.
    pub fn next_change_address_blocking(
        &self,
    ) -> Result<DashAddress, crate::error::PlatformWalletError> {
        self.next_change_address_for_account_blocking(0)
    }

    /// Blocking version of `next_change_address_for_account`.
    pub fn next_change_address_for_account_blocking(
        &self,
        account_index: u32,
    ) -> Result<DashAddress, crate::error::PlatformWalletError> {
        let xpub = {
            let wallet = self.wallet.blocking_read();
            let path = key_wallet::account::AccountType::Standard {
                index: account_index,
                standard_account_type: key_wallet::account::StandardAccountType::BIP44Account,
            }
            .derivation_path(wallet.network)
            .map_err(|e| crate::error::PlatformWalletError::WalletCreation(e.to_string()))?;
            wallet
                .derive_extended_public_key(&path)
                .map_err(|e| crate::error::PlatformWalletError::WalletCreation(e.to_string()))?
        };
        let mut info = self.wallet_info.blocking_write();
        let account = info
            .accounts
            .standard_bip44_accounts
            .get_mut(&account_index)
            .ok_or_else(|| {
                crate::error::PlatformWalletError::WalletCreation(format!(
                    "BIP-44 account {} not found",
                    account_index
                ))
            })?;
        account
            .next_change_address(Some(&xpub), true)
            .map_err(|e| crate::error::PlatformWalletError::WalletCreation(e.to_string()))
    }

    /// Get the next unused BIP-44 internal (change) address for a specific account.
    pub async fn next_change_address_for_account(
        &self,
        account_index: u32,
    ) -> Result<DashAddress, crate::error::PlatformWalletError> {
        let xpub = self.derive_account_xpub(account_index).await?;
        let mut info = self.wallet_info.write().await;
        let account = info
            .accounts
            .standard_bip44_accounts
            .get_mut(&account_index)
            .ok_or_else(|| {
                crate::error::PlatformWalletError::WalletCreation(format!(
                    "BIP-44 account {} not found",
                    account_index
                ))
            })?;
        account
            .next_change_address(Some(&xpub), true)
            .map_err(|e| crate::error::PlatformWalletError::WalletCreation(e.to_string()))
    }

    /// Get the network from the SDK.
    pub fn network(&self) -> key_wallet::Network {
        self.sdk.network
    }

    /// Derive the BIP-44 account-level extended public key at
    /// `m/44'/coin_type'/account_index'`.
    ///
    /// Uses `AccountType::Standard` to build the derivation path, matching
    /// the same approach used by the blocking address methods.
    async fn derive_account_xpub(
        &self,
        account_index: u32,
    ) -> Result<key_wallet::bip32::ExtendedPubKey, crate::error::PlatformWalletError> {
        let wallet = self.wallet.read().await;
        let path = key_wallet::account::AccountType::Standard {
            index: account_index,
            standard_account_type: key_wallet::account::StandardAccountType::BIP44Account,
        }
        .derivation_path(wallet.network)
        .map_err(|e| {
            crate::error::PlatformWalletError::WalletCreation(format!(
                "Invalid account index: {}",
                e
            ))
        })?;
        wallet.derive_extended_public_key(&path).map_err(|e| {
            crate::error::PlatformWalletError::WalletCreation(format!(
                "Failed to derive account xpub: {}",
                e
            ))
        })
    }
}

// Transaction status is tracked natively in key-wallet's TransactionRecord.context.

// ---------------------------------------------------------------------------
// Transaction broadcasting
// ---------------------------------------------------------------------------

impl CoreWallet {
    /// Broadcast a signed transaction to the network via DAPI.
    ///
    /// Serializes the transaction using consensus encoding and sends it
    /// through the SDK's DAPI client using the `BroadcastTransactionRequest`
    /// gRPC call.
    ///
    /// Returns the transaction ID on success.
    pub async fn broadcast_transaction(
        &self,
        transaction: &Transaction,
    ) -> Result<dashcore::Txid, PlatformWalletError> {
        use dash_sdk::dapi_client::{DapiRequestExecutor, IntoInner, RequestSettings};
        use dash_sdk::dapi_grpc::core::v0::BroadcastTransactionRequest;

        let tx_bytes = consensus::serialize(transaction);

        let request = BroadcastTransactionRequest {
            transaction: tx_bytes,
            allow_high_fees: false,
            bypass_limits: false,
        };

        let _response = self
            .sdk
            .execute(request, RequestSettings::default())
            .await
            .into_inner()
            .map_err(|e| {
                PlatformWalletError::TransactionBroadcast(format!("DAPI broadcast failed: {}", e))
            })?;

        Ok(transaction.txid())
    }
}

// ---------------------------------------------------------------------------
// Simple payment transaction
// ---------------------------------------------------------------------------

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
    /// 5. Broadcasts the transaction via DAPI.
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
        let spendable: Vec<Utxo> = {
            let info = self.wallet_info.read().await;
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
}

// ---------------------------------------------------------------------------
// Payment helpers
// ---------------------------------------------------------------------------

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
    // -- Private helpers -----------------------------------------------------

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
            // Assume outputs count = requested outputs + 1 change.
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

            // Update estimate and retry.
            fee_estimate = fee_with_change;
        }

        Err(PlatformWalletError::TransactionBuild(format!(
            "Insufficient funds after retry: need {} + fee {}",
            total_output, fee_estimate
        )))
    }

    /// Sign all inputs of a transaction using P2PKH.
    ///
    /// For each input, looks up the UTXO address, finds the corresponding
    /// derivation path in the wallet info, derives the private key, and
    /// constructs the scriptSig.
    ///
    /// This method is shared between asset lock and standard payment
    /// transaction building.
    async fn sign_transaction_inputs(
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

        // Look up derivation paths for all UTXO addresses.
        let derivation_paths = {
            let info = self.wallet_info.read().await;
            selected_utxos
                .iter()
                .map(|(_, _, address)| {
                    // Search all accounts for the address's derivation path.
                    for account in info.accounts.all_accounts() {
                        if let Some(path) = account.address_derivation_path(address) {
                            return Ok(path);
                        }
                    }
                    Err(PlatformWalletError::TransactionBuild(format!(
                        "Address {} not found in wallet",
                        address
                    )))
                })
                .collect::<Result<Vec<_>, _>>()?
        };

        // Derive private keys and sign.
        let wallet = self.wallet.read().await;
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

impl std::fmt::Debug for CoreWallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CoreWallet")
            .field("network", &self.sdk.network)
            .finish()
    }
}
