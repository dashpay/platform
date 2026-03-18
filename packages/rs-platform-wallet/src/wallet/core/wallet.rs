//! Core wallet functionality: balance, UTXOs, addresses, transaction history.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use dashcore::consensus;
use dashcore::secp256k1::{Message, Secp256k1};
use dashcore::sighash::SighashCache;
use dashcore::transaction::special_transaction::asset_lock::AssetLockPayload;
use dashcore::transaction::special_transaction::TransactionPayload;
use dashcore::Address as DashAddress;
use dashcore::{OutPoint, PrivateKey, ScriptBuf, Transaction, TxIn, TxOut};
use dpp::prelude::CoreBlockHeight;
use key_wallet::account::TransactionRecord;
use key_wallet::bip32::DerivationPath;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use key_wallet::{Network, Utxo, WalletCoreBalance};
use tokio::sync::RwLock;

use crate::error::PlatformWalletError;

use super::types::{CoreAccountSummary, CoreAddressInfo};

/// Core wallet providing UTXO, balance, and address functionality.
#[derive(Clone)]
pub struct CoreWallet {
    pub(crate) sdk: dash_sdk::Sdk,
    pub(crate) wallet: Arc<RwLock<Wallet>>,
    pub(crate) wallet_info: Arc<RwLock<ManagedWalletInfo>>,
    pub(crate) network: Network,
}

impl CoreWallet {
    /// Get the wallet balance (spendable, unconfirmed, total).
    pub async fn balance(&self) -> WalletCoreBalance {
        let info = self.wallet_info.read().await;
        info.balance()
    }

    /// Get all UTXOs.
    pub async fn utxos(&self) -> BTreeSet<Utxo> {
        let info = self.wallet_info.read().await;
        info.utxos().into_iter().cloned().collect()
    }

    /// Get spendable UTXOs (confirmed, non-dust, unlocked).
    pub async fn spendable_utxos(&self) -> BTreeSet<Utxo> {
        let info = self.wallet_info.read().await;
        info.get_spendable_utxos().into_iter().cloned().collect()
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
        let xpub = self.account_xpub(account_index).await?;
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

    /// Get the next unused change address for the default account.
    pub async fn next_change_address(
        &self,
    ) -> Result<DashAddress, crate::error::PlatformWalletError> {
        self.next_change_address_for_account(0).await
    }

    /// Get the next unused BIP-44 internal (change) address for a specific account.
    pub async fn next_change_address_for_account(
        &self,
        account_index: u32,
    ) -> Result<DashAddress, crate::error::PlatformWalletError> {
        let xpub = self.account_xpub(account_index).await?;
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

    /// Get all monitored addresses across all account types.
    pub async fn monitored_addresses(&self) -> Vec<DashAddress> {
        let info = self.wallet_info.read().await;
        info.monitored_addresses()
    }

    /// Get the current synced height.
    pub async fn synced_height(&self) -> CoreBlockHeight {
        let info = self.wallet_info.read().await;
        info.synced_height()
    }

    /// Get the wallet birth height.
    pub async fn birth_height(&self) -> CoreBlockHeight {
        let info = self.wallet_info.read().await;
        info.birth_height()
    }

    /// Get the cached network (sync, no lock needed).
    pub fn network(&self) -> Network {
        self.network
    }

    /// Get the transaction history.
    pub async fn transaction_history(&self) -> Vec<TransactionRecord> {
        let info = self.wallet_info.read().await;
        info.transaction_history().into_iter().cloned().collect()
    }

    /// Get immature transactions (coinbase outputs not yet mature).
    pub async fn immature_transactions(&self) -> Vec<Transaction> {
        let info = self.wallet_info.read().await;
        info.immature_transactions()
    }

    /// Get detailed info for every address across all accounts.
    ///
    /// Iterates all managed accounts and their address pools, building a
    /// [`CoreAddressInfo`] for each generated address. UTXO counts are
    /// computed by scanning the account's UTXO map.
    pub async fn all_address_info(&self) -> Vec<CoreAddressInfo> {
        let info = self.wallet_info.read().await;
        let mut result = Vec::new();

        for account in info.accounts.all_accounts() {
            let account_index = account.index();

            // Build a quick per-address UTXO count from the account's utxo map.
            let mut utxo_counts: BTreeMap<DashAddress, usize> = BTreeMap::new();
            for utxo in account.utxos.values() {
                *utxo_counts.entry(utxo.address.clone()).or_default() += 1;
            }

            for pool in account.account_type.address_pools() {
                for addr_info in pool.addresses.values() {
                    result.push(CoreAddressInfo {
                        address: addr_info.address.clone(),
                        derivation_path: addr_info.path.clone(),
                        balance: addr_info.balance,
                        total_received: addr_info.total_received,
                        utxo_count: utxo_counts
                            .get(&addr_info.address)
                            .copied()
                            .unwrap_or(0),
                        is_used: addr_info.used,
                        index: addr_info.index,
                        account_index,
                    });
                }
            }
        }

        result
    }

    /// Get detailed info for a single address, if it belongs to this wallet.
    ///
    /// Searches all accounts and their address pools for the given address.
    pub async fn address_info(&self, address: &DashAddress) -> Option<CoreAddressInfo> {
        let info = self.wallet_info.read().await;

        for account in info.accounts.all_accounts() {
            if let Some(addr_info) = account.get_address_info(address) {
                let utxo_count = account
                    .utxos
                    .values()
                    .filter(|u| &u.address == address)
                    .count();

                return Some(CoreAddressInfo {
                    address: addr_info.address.clone(),
                    derivation_path: addr_info.path.clone(),
                    balance: addr_info.balance,
                    total_received: addr_info.total_received,
                    utxo_count,
                    is_used: addr_info.used,
                    index: addr_info.index,
                    account_index: account.index(),
                });
            }
        }

        None
    }

    /// Get a summary for each managed account.
    ///
    /// Returns one [`CoreAccountSummary`] per account with aggregate
    /// balance, address count, and used-address count.
    pub async fn account_summaries(&self) -> Vec<CoreAccountSummary> {
        let info = self.wallet_info.read().await;

        info.accounts
            .all_accounts()
            .iter()
            .map(|account| CoreAccountSummary {
                account_index: account.index(),
                balance: account.balance,
                address_count: account.total_address_count(),
                used_address_count: account.used_address_count(),
            })
            .collect()
    }

    /// Get all UTXOs grouped by their owning address.
    ///
    /// Iterates every account's UTXO set and groups the entries by
    /// the address field.
    pub async fn utxos_by_address(&self) -> BTreeMap<DashAddress, Vec<Utxo>> {
        let info = self.wallet_info.read().await;
        let mut map: BTreeMap<DashAddress, Vec<Utxo>> = BTreeMap::new();

        for account in info.accounts.all_accounts() {
            for utxo in account.utxos.values() {
                map.entry(utxo.address.clone())
                    .or_default()
                    .push(utxo.clone());
            }
        }

        map
    }

    /// Get the extended public key for a specific account index.
    ///
    /// Derives the BIP-44 account-level key at `m/44'/coin_type'/account_index'`.
    pub async fn account_xpub(
        &self,
        account_index: u32,
    ) -> Result<key_wallet::bip32::ExtendedPubKey, crate::error::PlatformWalletError> {
        use key_wallet::bip32::{ChildNumber, DerivationPath};

        let coin_type = if self.network == Network::Mainnet {
            5u32 // DASH mainnet
        } else {
            1u32 // testnet/devnet/regtest all use coin_type 1
        };

        let path = DerivationPath::from(vec![
            ChildNumber::from_hardened_idx(44).expect("valid"),
            ChildNumber::from_hardened_idx(coin_type).expect("valid"),
            ChildNumber::from_hardened_idx(account_index).map_err(|e| {
                crate::error::PlatformWalletError::WalletCreation(format!(
                    "Invalid account index: {}",
                    e
                ))
            })?,
        ]);

        let wallet = self.wallet.read().await;
        wallet.derive_extended_public_key(&path).map_err(|e| {
            crate::error::PlatformWalletError::WalletCreation(format!(
                "Failed to derive account xpub: {}",
                e
            ))
        })
    }
}

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
                PlatformWalletError::TransactionBroadcast(format!(
                    "DAPI broadcast failed: {}",
                    e
                ))
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
// Asset lock transaction building
// ---------------------------------------------------------------------------

/// Minimum fee for an asset lock transaction (duffs).
const MIN_ASSET_LOCK_FEE: u64 = 3_000;

/// Minimum value for a change output (duffs). Outputs below this threshold are
/// considered dust and will be rejected by the network.
const DUST_THRESHOLD: u64 = 546;

/// Estimate the transaction size in bytes for an asset lock transaction.
///
/// Assumes P2PKH inputs (~148 B each), standard outputs (~34 B each),
/// a ~10 B header, and a ~60 B asset-lock payload.
fn estimate_tx_size(num_inputs: usize, num_outputs: usize) -> u64 {
    (10 + (num_inputs * 148) + (num_outputs * 34) + 60) as u64
}

/// Estimate the transaction size in bytes for a standard (non-special) transaction.
///
/// Assumes P2PKH inputs (~148 B each), standard outputs (~34 B each),
/// and a ~10 B header.
fn estimate_standard_tx_size(num_inputs: usize, num_outputs: usize) -> usize {
    10 + (num_inputs * 148) + (num_outputs * 34)
}

/// Result of asset lock fee calculation.
struct AssetLockFeeResult {
    /// Transaction fee in duffs. Retained for diagnostics and future use.
    #[allow(dead_code)]
    fee: u64,
    actual_amount: u64,
    change: Option<u64>,
}

/// Calculate fee, actual amount, and change for an asset lock transaction.
///
/// Uses an iterative approach: starts assuming a change output exists, then
/// recomputes if the change disappears under the real fee.
fn calculate_asset_lock_fee(
    total_input_value: u64,
    requested_amount: u64,
    num_inputs: usize,
) -> Result<AssetLockFeeResult, String> {
    // First pass: assume 2 outputs (1 burn + 1 change).
    let fee_with_change = std::cmp::max(MIN_ASSET_LOCK_FEE, estimate_tx_size(num_inputs, 2));

    let required_with_change = requested_amount
        .checked_add(fee_with_change)
        .ok_or("Overflow computing required amount + fee")?;
    let tentative_change = total_input_value.checked_sub(required_with_change);

    // If change exceeds dust threshold, include it as an output.
    if let Some(change) = tentative_change {
        if change >= DUST_THRESHOLD {
            return Ok(AssetLockFeeResult {
                fee: fee_with_change,
                actual_amount: requested_amount,
                change: Some(change),
            });
        }
    }

    // Change is zero or below dust under the 2-output fee.
    // Recompute with 1 output (no change).
    let fee_no_change = std::cmp::max(MIN_ASSET_LOCK_FEE, estimate_tx_size(num_inputs, 1));

    let required_no_change = requested_amount
        .checked_add(fee_no_change)
        .ok_or("Overflow computing required amount + fee")?;

    if total_input_value >= required_no_change {
        // Enough funds without a change output. Any leftover becomes additional fee.
        return Ok(AssetLockFeeResult {
            fee: total_input_value - requested_amount,
            actual_amount: requested_amount,
            change: None,
        });
    }

    Err(format!(
        "Insufficient funds: need {} + {} fee, have {}",
        requested_amount, fee_no_change, total_input_value
    ))
}

impl CoreWallet {
    // -- Public API ----------------------------------------------------------

    /// Build an asset lock transaction for identity registration.
    ///
    /// Derives the funding key at the DIP-9 registration path:
    /// `m/9'/coin_type'/5'/1'/identity_index'`
    ///
    /// Returns the signed transaction and the one-time private key whose
    /// corresponding public key is embedded in the asset lock payload.
    pub async fn build_registration_asset_lock_transaction(
        &self,
        amount_duffs: u64,
        identity_index: u32,
    ) -> Result<(Transaction, PrivateKey), PlatformWalletError> {
        let funding_path =
            DerivationPath::identity_registration_path(self.network, identity_index);
        self.build_asset_lock_transaction(amount_duffs, &funding_path)
            .await
    }

    /// Build an asset lock transaction for identity top-up.
    ///
    /// Derives the funding key at the DIP-9 top-up path:
    /// `m/9'/coin_type'/5'/2'/identity_index'/topup_index`
    ///
    /// Returns the signed transaction and the one-time private key whose
    /// corresponding public key is embedded in the asset lock payload.
    pub async fn build_topup_asset_lock_transaction(
        &self,
        amount_duffs: u64,
        identity_index: u32,
        topup_index: u32,
    ) -> Result<(Transaction, PrivateKey), PlatformWalletError> {
        let funding_path =
            DerivationPath::identity_top_up_path(self.network, identity_index, topup_index);
        self.build_asset_lock_transaction(amount_duffs, &funding_path)
            .await
    }

    /// Build an asset lock transaction using the given DIP-9 funding key path.
    ///
    /// This is the shared implementation for both registration and top-up.
    /// The caller provides the full derivation path for the one-time funding
    /// key that will appear in the asset lock payload's `credit_outputs`.
    ///
    /// # Steps
    ///
    /// 1. Derive the one-time private key from the wallet at `funding_key_path`.
    /// 2. Select spendable UTXOs covering `amount_duffs + estimated_fee`.
    /// 3. Build a v3 special transaction with:
    ///    - Output 0: `OP_RETURN` burn (value = actual amount).
    ///    - Output 1 (optional): change back to the wallet.
    ///    - `AssetLockPayload` with a single credit output (P2PKH to the
    ///      one-time key).
    /// 4. Sign each input using the private key looked up from the wallet for
    ///    the UTXO's owning address.
    /// 5. Return the signed transaction and the one-time private key.
    pub async fn build_asset_lock_transaction(
        &self,
        amount_duffs: u64,
        funding_key_path: &DerivationPath,
    ) -> Result<(Transaction, PrivateKey), PlatformWalletError> {
        if amount_duffs == 0 {
            return Err(PlatformWalletError::AssetLockTransaction(
                "Amount must be greater than zero".to_string(),
            ));
        }

        let secp = Secp256k1::new();

        // 1. Derive the one-time funding key.
        let one_time_private_key = {
            let wallet = self.wallet.read().await;
            let extended_key = wallet
                .derive_extended_private_key(funding_key_path)
                .map_err(|e| {
                    PlatformWalletError::AssetLockTransaction(format!(
                        "Failed to derive funding key: {}",
                        e
                    ))
                })?;
            extended_key.to_priv()
        };

        let one_time_public_key = one_time_private_key.public_key(&secp);
        let one_time_key_hash = one_time_public_key.pubkey_hash();

        // 2. Select spendable UTXOs.
        let (selected_utxos, fee_result) = {
            let info = self.wallet_info.read().await;
            let spendable: Vec<Utxo> = info.get_spendable_utxos().into_iter().cloned().collect();

            if spendable.is_empty() {
                return Err(PlatformWalletError::AssetLockTransaction(
                    "No spendable UTXOs available".to_string(),
                ));
            }

            self.select_utxos_and_compute_fee(spendable, amount_duffs)?
        };

        let actual_amount = fee_result.actual_amount;

        // 3. Build the transaction.

        // Credit output: P2PKH to the one-time key (this goes into the payload,
        // not the transaction outputs).
        let payload_output = TxOut {
            value: actual_amount,
            script_pubkey: ScriptBuf::new_p2pkh(&one_time_key_hash),
        };

        // Burn output: OP_RETURN
        let burn_output = TxOut {
            value: actual_amount,
            script_pubkey: ScriptBuf::new_op_return(&[]),
        };

        let payload = AssetLockPayload {
            version: 1,
            credit_outputs: vec![payload_output],
        };

        // Build outputs: burn first, then optional change.
        let mut outputs = vec![burn_output];

        let change_address = if let Some(change_value) = fee_result.change {
            let addr = self.next_change_address().await?;
            outputs.push(TxOut {
                value: change_value,
                script_pubkey: addr.script_pubkey(),
            });
            Some(addr)
        } else {
            None
        };
        let _ = change_address; // will be useful later for UTXO tracking

        // Build inputs from the selected UTXOs.
        let inputs: Vec<TxIn> = selected_utxos
            .iter()
            .map(|(outpoint, _, _)| TxIn {
                previous_output: *outpoint,
                ..Default::default()
            })
            .collect();

        let mut tx = Transaction {
            version: 3,
            lock_time: 0,
            input: inputs,
            output: outputs,
            special_transaction_payload: Some(TransactionPayload::AssetLockPayloadType(payload)),
        };

        // 4. Sign each input.
        self.sign_transaction_inputs(&secp, &mut tx, &selected_utxos)
            .await?;

        Ok((tx, one_time_private_key))
    }

    /// Build and broadcast an asset lock transaction for identity registration.
    /// Build, broadcast, and wait for an asset lock proof for identity registration.
    ///
    /// This is a convenience method that combines:
    /// 1. Building and broadcasting the registration asset lock transaction.
    /// 2. Subscribing to the transaction stream via DAPI.
    /// 3. Waiting for an instant-send lock or chain-lock proof.
    ///
    /// Returns the asset lock proof and the one-time private key whose
    /// corresponding public key is embedded in the asset lock payload.
    pub async fn create_registration_asset_lock_proof(
        &self,
        amount_duffs: u64,
        identity_index: u32,
    ) -> Result<(dpp::prelude::AssetLockProof, PrivateKey), PlatformWalletError> {
        let (tx, key) = self
            .build_registration_asset_lock_transaction(amount_duffs, identity_index)
            .await?;

        let proof = self.broadcast_and_wait_for_asset_lock_proof(&tx, &key).await?;

        Ok((proof, key))
    }

    /// Build, broadcast, and wait for an asset lock proof for identity top-up.
    ///
    /// This is a convenience method that combines:
    /// 1. Building and broadcasting the top-up asset lock transaction.
    /// 2. Subscribing to the transaction stream via DAPI.
    /// 3. Waiting for an instant-send lock or chain-lock proof.
    ///
    /// Returns the asset lock proof and the one-time private key whose
    /// corresponding public key is embedded in the asset lock payload.
    pub async fn create_topup_asset_lock_proof(
        &self,
        amount_duffs: u64,
        identity_index: u32,
        topup_index: u32,
    ) -> Result<(dpp::prelude::AssetLockProof, PrivateKey), PlatformWalletError> {
        let (tx, key) = self
            .build_topup_asset_lock_transaction(amount_duffs, identity_index, topup_index)
            .await?;

        let proof = self.broadcast_and_wait_for_asset_lock_proof(&tx, &key).await?;

        Ok((proof, key))
    }

    /// Broadcast an asset lock transaction and wait for its proof.
    ///
    /// Performs the following steps:
    /// 1. Fetches the current best block hash via `GetBlockchainStatusRequest`.
    /// 2. Derives the one-time key's P2PKH address for the bloom filter.
    /// 3. Opens a transaction stream subscription (before broadcasting, to
    ///    avoid missing the instant-send lock).
    /// 4. Broadcasts the transaction via DAPI.
    /// 5. Waits for an instant-send lock or chain-lock proof on the stream.
    async fn broadcast_and_wait_for_asset_lock_proof(
        &self,
        transaction: &Transaction,
        one_time_private_key: &PrivateKey,
    ) -> Result<dpp::prelude::AssetLockProof, PlatformWalletError> {
        use dash_sdk::dapi_client::{DapiRequestExecutor, IntoInner, RequestSettings};
        use dash_sdk::dapi_grpc::core::v0::GetBlockchainStatusRequest;
        use std::time::Duration;

        let secp = Secp256k1::new();

        // 1. Get the best block hash for the stream subscription.
        let status_response = self
            .sdk
            .execute(GetBlockchainStatusRequest {}, RequestSettings::default())
            .await
            .into_inner()
            .map_err(|e| {
                PlatformWalletError::AssetLockProofWait(format!(
                    "Failed to get blockchain status: {}",
                    e
                ))
            })?;

        let best_block_hash = status_response
            .chain
            .ok_or_else(|| {
                PlatformWalletError::AssetLockProofWait(
                    "Blockchain status missing chain info".to_string(),
                )
            })?
            .best_block_hash;

        // 2. Derive the one-time key's P2PKH address for the bloom filter.
        let one_time_public_key = one_time_private_key.public_key(&secp);
        let asset_lock_address = DashAddress::p2pkh(&one_time_public_key, self.network);

        // 3. Start the instant-send lock stream BEFORE broadcasting to avoid
        //    missing the proof.
        let stream = self
            .sdk
            .start_instant_send_lock_stream(best_block_hash, &asset_lock_address)
            .await
            .map_err(|e| {
                PlatformWalletError::AssetLockProofWait(format!(
                    "Failed to start instant-send lock stream: {}",
                    e
                ))
            })?;

        // 4. Broadcast the transaction.
        self.broadcast_transaction(transaction).await?;

        // 5. Wait for the asset lock proof with a 5-minute timeout.
        let proof = self
            .sdk
            .wait_for_asset_lock_proof_for_transaction(
                stream,
                transaction,
                Some(Duration::from_secs(300)),
            )
            .await
            .map_err(|e| {
                PlatformWalletError::AssetLockProofWait(format!(
                    "Failed to receive asset lock proof: {}",
                    e
                ))
            })?;

        Ok(proof)
    }

    // -- Private helpers -----------------------------------------------------

    /// Select UTXOs covering `amount + fee`, retrying once if the initial fee
    /// estimate was too low.
    ///
    /// Returns a vec of `(OutPoint, TxOut, DashAddress)` for the selected UTXOs
    /// and the fee calculation result.
    fn select_utxos_and_compute_fee(
        &self,
        mut spendable: Vec<Utxo>,
        amount: u64,
    ) -> Result<(Vec<(OutPoint, TxOut, DashAddress)>, AssetLockFeeResult), PlatformWalletError>
    {
        // Sort by value descending so we greedily select fewest UTXOs.
        spendable.sort_by(|a, b| b.value().cmp(&a.value()));

        let mut fee_estimate = MIN_ASSET_LOCK_FEE;

        for _ in 0..2 {
            let target = amount.saturating_add(fee_estimate);

            let mut selected = Vec::new();
            let mut total_input = 0u64;

            for utxo in &spendable {
                if total_input >= target {
                    break;
                }
                selected.push((
                    utxo.outpoint,
                    utxo.txout.clone(),
                    utxo.address.clone(),
                ));
                total_input += utxo.value();
            }

            if total_input < amount.saturating_add(MIN_ASSET_LOCK_FEE) {
                return Err(PlatformWalletError::AssetLockTransaction(format!(
                    "Insufficient funds: need {} + fee, have {}",
                    amount, total_input
                )));
            }

            match calculate_asset_lock_fee(total_input, amount, selected.len()) {
                Ok(fee_result) => return Ok((selected, fee_result)),
                Err(_) if fee_estimate == MIN_ASSET_LOCK_FEE => {
                    // Real fee exceeds initial estimate. Recompute with a better
                    // estimate and retry so we can pick up additional UTXOs.
                    fee_estimate = std::cmp::max(
                        MIN_ASSET_LOCK_FEE,
                        estimate_tx_size(selected.len(), 2),
                    );
                    continue;
                }
                Err(e) => {
                    return Err(PlatformWalletError::AssetLockTransaction(e));
                }
            }
        }

        Err(PlatformWalletError::AssetLockTransaction(format!(
            "Insufficient funds after retry: need {} + fee {}",
            amount, fee_estimate
        )))
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
                selected.push((
                    utxo.outpoint,
                    utxo.txout.clone(),
                    utxo.address.clone(),
                ));
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
            let fee_with_change =
                std::cmp::max(MIN_ASSET_LOCK_FEE, estimate_standard_tx_size(selected.len(), num_payment_outputs + 1) as u64);
            let tentative_change = total_input
                .checked_sub(total_output)
                .and_then(|r| r.checked_sub(fee_with_change));

            if let Some(change) = tentative_change {
                if change >= DUST_THRESHOLD {
                    return Ok((selected, fee_with_change, Some(change)));
                }
            }

            // No change (or dust): recompute fee without change output.
            let fee_no_change =
                std::cmp::max(MIN_ASSET_LOCK_FEE, estimate_standard_tx_size(selected.len(), num_payment_outputs) as u64);

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
            .field("network", &self.network)
            .finish()
    }
}
