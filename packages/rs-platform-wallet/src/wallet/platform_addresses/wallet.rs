//! Platform address wallet for DIP-17 platform payment addresses.

use std::collections::BTreeMap;
use std::sync::Arc;

use dpp::address_funds::{AddressFundsFeeStrategyStep, AddressWitness, PlatformAddress};
use dpp::fee::Credits;
use dpp::identity::core_script::CoreScript;
use dpp::identity::signer::Signer;
use dpp::platform_value::BinaryData;
use dpp::withdrawal::Pooling;
use dpp::ProtocolError;
use key_wallet::PlatformP2PKHAddress;
use tokio::sync::RwLock;
use zeroize::Zeroizing;

use dashcore::PrivateKey;
use dpp::identity::state_transition::asset_lock_proof::AssetLockProof;

use crate::changeset::PlatformAddressChangeSet;
use crate::error::PlatformWalletError;
use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
use dash_sdk::platform::address_sync::AddressSyncResult;
use dash_sdk::platform::transition::address_credit_withdrawal::WithdrawAddressFunds;
use dash_sdk::platform::transition::top_up_address::TopUpAddress;
use dash_sdk::platform::transition::transfer_address_funds::TransferAddressFunds;
use key_wallet_manager::WalletManager;

use super::provider::PlatformPaymentAddressProvider;

/// Platform address wallet providing DIP-17 platform payment address functionality.
#[derive(Clone)]
pub struct PlatformAddressWallet {
    pub(crate) sdk: Arc<dash_sdk::Sdk>,
    /// The shared wallet manager lock for all mutable wallet state.
    pub(crate) wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    /// Identifies which wallet within the manager this sub-wallet operates on.
    pub(crate) wallet_id: WalletId,
}

impl PlatformAddressWallet {
    /// Create a new PlatformAddressWallet.
    pub(crate) fn new(
        sdk: Arc<dash_sdk::Sdk>,
        wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        wallet_id: WalletId,
    ) -> Self {
        Self {
            sdk,
            wallet_manager,
            wallet_id,
        }
    }

    /// Get the network from the SDK.
    pub fn network(&self) -> key_wallet::Network {
        self.sdk.network
    }

    /// Sync platform address balances from Platform.
    ///
    /// Uses the SDK's privacy-preserving trunk/branch address synchronization
    /// with DIP-17 address discovery via gap limit scanning.
    ///
    /// Returns both the raw [`AddressSyncResult`] and a
    /// [`PlatformAddressChangeSet`] describing every address update /
    /// tombstone caused by the sync.
    pub async fn sync_balances(
        &self,
    ) -> Result<(AddressSyncResult, PlatformAddressChangeSet), PlatformWalletError> {
        // Build the address provider from the wallet.
        let mut provider = PlatformPaymentAddressProvider::from_wallet(
            self.wallet_manager.clone(),
            self.wallet_id,
            self.sdk.network,
        )
        .map_err(|e| {
            PlatformWalletError::AddressSync(format!("Failed to create address provider: {}", e))
        })?;

        let result = self
            .sdk
            .sync_address_balances(&mut provider, None, None)
            .await?;

        // Update cached balances from the sync results.
        let mut wm = self.wallet_manager.write().await;
        let info = wm.get_wallet_info_mut(&self.wallet_id).ok_or_else(|| {
            PlatformWalletError::AddressSync("Wallet not found in wallet manager".to_string())
        })?;

        // A sync replaces the whole cached set. Pre-seed the tombstone
        // set with every previously-cached address; the inserts below
        // remove any address that reappears in the fresh results, so
        // what's left in `removed` is exactly the drained set.
        let mut cs = PlatformAddressChangeSet {
            removed: info.platform_address_balances.keys().copied().collect(),
            ..Default::default()
        };
        info.platform_address_balances.clear();
        for ((_, key), funds) in &result.found {
            match PlatformAddress::from_bytes(key) {
                Ok(platform_addr) => {
                    info.platform_address_balances
                        .insert(platform_addr, funds.balance);
                    cs.addresses.insert(platform_addr, funds.balance);
                    cs.removed.remove(&platform_addr);
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to parse PlatformAddress from sync result key: {}",
                        e
                    );
                }
            }
        }

        Ok((result, cs))
    }

    /// Transfer credits between platform addresses.
    ///
    /// Broadcasts an address funds transfer state transition. The fee is deducted
    /// from the first input address by default.
    pub async fn transfer(
        &self,
        inputs: BTreeMap<PlatformAddress, Credits>,
        outputs: BTreeMap<PlatformAddress, Credits>,
    ) -> Result<PlatformAddressChangeSet, PlatformWalletError> {
        if inputs.is_empty() {
            return Err(PlatformWalletError::AddressOperation(
                "Transfer requires at least one input address".to_string(),
            ));
        }

        let fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];

        let address_infos = self
            .sdk
            .transfer_address_funds(inputs, outputs, fee_strategy, self, None)
            .await?;

        // Update cached balances from the proof-verified response.
        let mut wm = self.wallet_manager.write().await;
        let mut cs = PlatformAddressChangeSet::default();
        if let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) {
            for (addr, maybe_info) in address_infos.iter() {
                match maybe_info {
                    Some(ai) => {
                        info.platform_address_balances.insert(*addr, ai.balance);
                        cs.addresses.insert(*addr, ai.balance);
                    }
                    None => {
                        info.platform_address_balances.remove(addr);
                        cs.removed.insert(*addr);
                    }
                }
            }
        }

        Ok(cs)
    }

    /// Withdraw platform credits to a Core L1 address.
    ///
    /// Broadcasts an address credit withdrawal state transition. The fee is deducted
    /// from the first input address by default.
    pub async fn withdraw(
        &self,
        inputs: BTreeMap<PlatformAddress, Credits>,
        output_script: CoreScript,
        core_fee_per_byte: u32,
    ) -> Result<PlatformAddressChangeSet, PlatformWalletError> {
        if inputs.is_empty() {
            return Err(PlatformWalletError::AddressOperation(
                "Withdrawal requires at least one input address".to_string(),
            ));
        }

        // Validate that the output script is a supported type (P2PKH or P2SH).
        if !output_script.is_p2pkh() && !output_script.is_p2sh() {
            return Err(PlatformWalletError::AddressOperation(
                "Output script must be P2PKH or P2SH".to_string(),
            ));
        }

        let fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];

        let address_infos = self
            .sdk
            .withdraw_address_funds(
                inputs,
                None, // No change output
                fee_strategy,
                core_fee_per_byte,
                Pooling::Never,
                output_script,
                self,
                None,
            )
            .await?;

        // Update cached balances from the proof-verified response.
        let mut wm = self.wallet_manager.write().await;
        let mut cs = PlatformAddressChangeSet::default();
        if let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) {
            for (addr, maybe_info) in address_infos.iter() {
                match maybe_info {
                    Some(ai) => {
                        info.platform_address_balances.insert(*addr, ai.balance);
                        cs.addresses.insert(*addr, ai.balance);
                    }
                    None => {
                        info.platform_address_balances.remove(addr);
                        cs.removed.insert(*addr);
                    }
                }
            }
        }

        Ok(cs)
    }

    /// Get all platform addresses with their cached balances.
    ///
    /// Returns the balances from the last call to [`sync_balances`](Self::sync_balances),
    /// [`transfer`](Self::transfer), or [`withdraw`](Self::withdraw).
    pub async fn addresses_with_balances(&self) -> Vec<(PlatformAddress, Credits)> {
        let wm = self.wallet_manager.read().await;
        wm.get_wallet_info(&self.wallet_id)
            .map(|info| {
                info.platform_address_balances
                    .iter()
                    .map(|(addr, &bal)| (*addr, bal))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get total platform credits across all addresses.
    ///
    /// Returns the sum of all cached balances.
    pub async fn total_credits(&self) -> Credits {
        let wm = self.wallet_manager.read().await;
        wm.get_wallet_info(&self.wallet_id)
            .map(|info| info.platform_address_balances.values().sum())
            .unwrap_or(0)
    }

    /// Fund platform addresses from a Core L1 asset lock.
    ///
    /// Broadcasts a top-up-address state transition that converts locked Dash
    /// into platform credits on the specified addresses. The fee is deducted
    /// from the first input address by default.
    ///
    /// # Arguments
    ///
    /// * `addresses` - Platform addresses to fund (with current balances for nonce lookup).
    /// * `asset_lock_proof` - Proof of the asset lock transaction on Core chain.
    /// * `asset_lock_private_key` - Private key corresponding to the asset lock.
    pub async fn fund_from_asset_lock(
        &self,
        addresses: BTreeMap<PlatformAddress, Option<Credits>>,
        asset_lock_proof: AssetLockProof,
        asset_lock_private_key: PrivateKey,
    ) -> Result<PlatformAddressChangeSet, PlatformWalletError> {
        if addresses.is_empty() {
            return Err(PlatformWalletError::AddressOperation(
                "fund_from_asset_lock requires at least one address".to_string(),
            ));
        }

        let fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];

        let address_infos = addresses
            .top_up(
                &self.sdk,
                asset_lock_proof,
                asset_lock_private_key,
                fee_strategy,
                self,
                None, // settings
            )
            .await?;

        // Update cached balances from the proof-verified response.
        let mut wm = self.wallet_manager.write().await;
        let mut cs = PlatformAddressChangeSet::default();
        if let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) {
            for (addr, maybe_info) in address_infos.iter() {
                match maybe_info {
                    Some(ai) => {
                        info.platform_address_balances.insert(*addr, ai.balance);
                        cs.addresses.insert(*addr, ai.balance);
                    }
                    None => {
                        info.platform_address_balances.remove(addr);
                        cs.removed.insert(*addr);
                    }
                }
            }
        }

        Ok(cs)
    }

    /// Find the private key for a platform address by searching all platform
    /// payment accounts in the wallet info.
    ///
    /// Returns the raw private key bytes wrapped in [`Zeroizing`] so they are
    /// automatically wiped from memory when the value is dropped.
    fn find_private_key_for_platform_address(
        &self,
        platform_address: &PlatformAddress,
    ) -> Result<Zeroizing<[u8; 32]>, ProtocolError> {
        let PlatformAddress::P2pkh(hash) = platform_address else {
            return Err(ProtocolError::Generic(
                "Only P2PKH Platform addresses are currently supported for signing".to_string(),
            ));
        };

        let target = PlatformP2PKHAddress::new(*hash);

        // Find the derivation path and derive the private key under a single lock.
        let wm = self.wallet_manager.blocking_read();
        let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
            ProtocolError::Generic("Wallet not found in wallet manager".to_string())
        })?;
        let wallet = wm.get_wallet(&self.wallet_id).ok_or_else(|| {
            ProtocolError::Generic("Wallet not found in wallet manager".to_string())
        })?;
        let mut found_path = None;
        for account in info.core_wallet.accounts.platform_payment_accounts.values() {
            for addr_info in account.addresses.addresses.values() {
                let Ok(pool_addr) = PlatformP2PKHAddress::from_address(&addr_info.address) else {
                    continue;
                };
                if pool_addr == target {
                    found_path = Some(addr_info.path.clone());
                    break;
                }
            }
            if found_path.is_some() {
                break;
            }
        }

        let path = found_path.ok_or_else(|| {
            ProtocolError::Generic(format!(
                "Platform address {:?} not found in wallet",
                platform_address
            ))
        })?;

        let secret_key = wallet.derive_private_key(&path).map_err(|e| {
            ProtocolError::Generic(format!(
                "Failed to derive private key for platform address: {}",
                e
            ))
        })?;

        Ok(Zeroizing::new(secret_key.secret_bytes()))
    }
}

impl Signer<PlatformAddress> for PlatformAddressWallet {
    fn sign(
        &self,
        platform_address: &PlatformAddress,
        data: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        let private_key_bytes = self.find_private_key_for_platform_address(platform_address)?;

        let signature = dashcore::signer::sign(data, private_key_bytes.as_ref())
            .map_err(|e| ProtocolError::Generic(format!("Failed to sign: {}", e)))?;

        Ok(BinaryData::new(signature.to_vec()))
    }

    fn sign_create_witness(
        &self,
        platform_address: &PlatformAddress,
        data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        let private_key_bytes = self.find_private_key_for_platform_address(platform_address)?;

        let signature = dashcore::signer::sign(data, private_key_bytes.as_ref())
            .map_err(|e| ProtocolError::Generic(format!("Failed to sign: {}", e)))?;

        Ok(AddressWitness::P2pkh {
            signature: BinaryData::new(signature.to_vec()),
        })
    }

    fn can_sign_with(&self, platform_address: &PlatformAddress) -> bool {
        self.find_private_key_for_platform_address(platform_address)
            .is_ok()
    }
}

impl std::fmt::Debug for PlatformAddressWallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlatformAddressWallet")
            .field("network", &self.sdk.network)
            .finish()
    }
}
