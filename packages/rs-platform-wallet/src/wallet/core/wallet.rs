//! Core wallet functionality: balance, UTXOs, addresses, transaction history.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use dashcore::consensus;
use dashcore::secp256k1::{Message, Secp256k1};
use dashcore::sighash::SighashCache;
use dashcore::Address as DashAddress;
use dashcore::{OutPoint, PrivateKey, ScriptBuf, Transaction, TxIn, TxOut};
use dpp::prelude::CoreBlockHeight;
use key_wallet::account::TransactionRecord;
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::{
    AssetLockFundingType, CreditOutputFunding,
};
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use key_wallet::{Utxo, WalletCoreBalance};
use tokio::sync::RwLock;

use crate::error::PlatformWalletError;

use super::types::{CoreAccountSummary, CoreAddressInfo};

use crate::events::TransactionStatus;
use dashcore::Txid;

use super::asset_lock::{AssetLockStatus, TrackedAssetLock};

/// Core wallet providing UTXO, balance, and address functionality.
#[derive(Clone)]
pub struct CoreWallet {
    pub(crate) sdk: dash_sdk::Sdk,
    pub(crate) wallet: Arc<RwLock<Wallet>>,
    pub(crate) wallet_info: Arc<RwLock<ManagedWalletInfo>>,
    /// Per-transaction finality status tracking.
    pub(crate) transaction_statuses: Arc<RwLock<BTreeMap<Txid, TransactionStatus>>>,
    /// Tracked asset lock transactions and their lifecycle status.
    pub(crate) tracked_asset_locks: Arc<RwLock<Vec<TrackedAssetLock>>>,
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

    /// Get the network from the SDK.
    pub fn network(&self) -> key_wallet::Network {
        self.sdk.network
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
                        utxo_count: utxo_counts.get(&addr_info.address).copied().unwrap_or(0),
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

        let coin_type = if self.sdk.network == key_wallet::Network::Mainnet {
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
// Transaction status tracking
// ---------------------------------------------------------------------------

impl CoreWallet {
    /// Get the finality status of a tracked transaction.
    pub async fn transaction_status(&self, txid: &Txid) -> Option<TransactionStatus> {
        let statuses = self.transaction_statuses.read().await;
        statuses.get(txid).copied()
    }

    /// Get all tracked transaction statuses.
    pub async fn all_transaction_statuses(&self) -> BTreeMap<Txid, TransactionStatus> {
        let statuses = self.transaction_statuses.read().await;
        statuses.clone()
    }

    /// Update a transaction's status. Returns the old status if the transaction
    /// was already tracked and the status actually changed (new > old).
    /// Returns `None` if no change occurred.
    pub(crate) async fn update_transaction_status(
        &self,
        txid: Txid,
        new_status: TransactionStatus,
    ) -> Option<TransactionStatus> {
        let mut statuses = self.transaction_statuses.write().await;
        let old_status = statuses.get(&txid).copied();
        match old_status {
            Some(old) if new_status > old => {
                statuses.insert(txid, new_status);
                Some(old)
            }
            None => {
                statuses.insert(txid, new_status);
                None
            }
            _ => None, // new_status <= old, no change
        }
    }
}

// ---------------------------------------------------------------------------
// Asset lock tracking
// ---------------------------------------------------------------------------

impl CoreWallet {
    /// Track a new asset lock transaction.
    pub async fn track_asset_lock(&self, lock: TrackedAssetLock) {
        let mut locks = self.tracked_asset_locks.write().await;
        locks.push(lock);
    }

    /// Return all asset locks that have not been consumed (status is not Used*).
    pub async fn unused_asset_locks(&self) -> Vec<TrackedAssetLock> {
        let locks = self.tracked_asset_locks.read().await;
        locks
            .iter()
            .filter(|l| !l.status.is_used())
            .cloned()
            .collect()
    }

    /// Mark an asset lock as used for registration or top-up.
    pub async fn mark_asset_lock_used(&self, txid: &Txid, usage: AssetLockStatus) {
        let mut locks = self.tracked_asset_locks.write().await;
        if let Some(lock) = locks.iter_mut().find(|l| &l.txid == txid) {
            lock.status = usage;
        }
    }

    /// Update the proof on a tracked asset lock (e.g. when IS or CL arrives).
    pub async fn update_asset_lock_proof(&self, txid: &Txid, proof: dpp::prelude::AssetLockProof) {
        let mut locks = self.tracked_asset_locks.write().await;
        if let Some(lock) = locks.iter_mut().find(|l| &l.txid == txid) {
            lock.proof = Some(proof);
        }
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
// Asset lock transaction building
// ---------------------------------------------------------------------------

/// Minimum fee for an asset lock transaction (duffs).
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

/// Default fee rate in duffs per kilobyte for asset lock transactions.
const DEFAULT_FEE_PER_KB: u64 = 1000;

impl CoreWallet {
    /// Build an asset lock transaction using the key-wallet builder.
    ///
    /// Delegates UTXO selection, fee calculation, change handling, and signing
    /// to `ManagedWalletInfo::build_asset_lock`.
    ///
    /// # Arguments
    ///
    /// * `amount_duffs` — Amount to lock in duffs.
    /// * `funding_type` — Which account to derive the one-time key from
    ///   (e.g., `IdentityRegistration`, `IdentityTopUp`).
    /// * `identity_index` — Identity index (used by `IdentityTopUp`, ignored by others).
    pub async fn build_asset_lock_transaction(
        &self,
        amount_duffs: u64,
        funding_type: AssetLockFundingType,
        identity_index: u32,
    ) -> Result<(Transaction, PrivateKey), PlatformWalletError> {
        if amount_duffs == 0 {
            return Err(PlatformWalletError::AssetLockTransaction(
                "Amount must be greater than zero".to_string(),
            ));
        }

        let wallet = self.wallet.read().await;
        let mut wallet_info = self.wallet_info.write().await;

        // 1. Peek at the next unused address from the funding account to
        //    build the credit output P2PKH script.
        let funding_address = Self::peek_next_funding_address(
            &mut wallet_info,
            &wallet,
            funding_type,
            identity_index,
        )?;

        // 2. Build the credit output for the asset lock payload.
        let credit_output = TxOut {
            value: amount_duffs,
            script_pubkey: funding_address.script_pubkey(),
        };

        let funding = CreditOutputFunding {
            output: credit_output,
            funding_type,
            identity_index,
        };

        // 3. Delegate to the key-wallet builder (account 0 for UTXOs).
        let result = wallet_info
            .build_asset_lock(&wallet, 0, vec![funding], DEFAULT_FEE_PER_KB)
            .map_err(|e| {
                PlatformWalletError::AssetLockTransaction(format!(
                    "Asset lock builder failed: {}",
                    e
                ))
            })?;

        // 4. Convert the raw key bytes to a PrivateKey.
        let key_bytes = result.keys.into_iter().next().ok_or_else(|| {
            PlatformWalletError::AssetLockTransaction(
                "Builder returned no keys".to_string(),
            )
        })?;
        let one_time_private_key =
            PrivateKey::from_byte_array(&key_bytes, self.sdk.network).map_err(|e| {
                PlatformWalletError::AssetLockTransaction(format!(
                    "Invalid private key from builder: {}",
                    e
                ))
            })?;

        Ok((result.transaction, one_time_private_key))
    }

    /// Peek at the next unused address from a funding account without
    /// consuming it (i.e. without marking it as used).
    ///
    /// The key-wallet builder's `next_private_key` will later find the same
    /// address, derive the private key, and mark it as used.
    fn peek_next_funding_address(
        wallet_info: &mut ManagedWalletInfo,
        wallet: &Wallet,
        funding_type: AssetLockFundingType,
        identity_index: u32,
    ) -> Result<DashAddress, PlatformWalletError> {
        let (managed_account, account_xpub) = match funding_type {
            AssetLockFundingType::IdentityRegistration => {
                let xpub = wallet
                    .accounts
                    .identity_registration
                    .as_ref()
                    .map(|a| a.account_xpub);
                let account = wallet_info
                    .accounts
                    .identity_registration
                    .as_mut()
                    .ok_or_else(|| {
                        PlatformWalletError::AssetLockTransaction(
                            "Identity registration account not found".to_string(),
                        )
                    })?;
                (account, xpub)
            }
            AssetLockFundingType::IdentityTopUp => {
                let xpub = wallet
                    .accounts
                    .identity_topup
                    .get(&identity_index)
                    .map(|a| a.account_xpub);
                let account = wallet_info
                    .accounts
                    .identity_topup
                    .get_mut(&identity_index)
                    .ok_or_else(|| {
                        PlatformWalletError::AssetLockTransaction(format!(
                            "Identity top-up account for index {} not found",
                            identity_index
                        ))
                    })?;
                (account, xpub)
            }
            other => {
                return Err(PlatformWalletError::AssetLockTransaction(format!(
                    "Unsupported funding type for asset lock: {:?}",
                    other
                )));
            }
        };

        // Get the next unused address from the pool.  We pass
        // `add_to_state: true` so that a newly-generated address is stored
        // in the pool and the builder's `next_private_key` can find it.
        // The address is NOT marked as used yet — that happens inside the
        // builder after a successful transaction build.
        managed_account
            .next_address(account_xpub.as_ref(), true)
            .map_err(|e| {
                PlatformWalletError::AssetLockTransaction(format!(
                    "Failed to get next funding address: {}",
                    e
                ))
            })
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
            .build_asset_lock_transaction(amount_duffs, AssetLockFundingType::IdentityRegistration, identity_index)
            .await?;

        let proof = self
            .broadcast_and_wait_for_asset_lock_proof(&tx, &key)
            .await?;

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
            .build_asset_lock_transaction(amount_duffs, AssetLockFundingType::IdentityTopUp, identity_index)
            .await?;

        let proof = self
            .broadcast_and_wait_for_asset_lock_proof(&tx, &key)
            .await?;

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
        let asset_lock_address = DashAddress::p2pkh(&one_time_public_key, self.sdk.network);

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
