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
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use key_wallet::{Network, PlatformP2PKHAddress};
use tokio::sync::RwLock;
use zeroize::Zeroizing;

use crate::error::PlatformWalletError;
use dash_sdk::platform::address_sync::AddressSyncResult;
use dash_sdk::platform::transition::address_credit_withdrawal::WithdrawAddressFunds;
use dash_sdk::platform::transition::transfer_address_funds::TransferAddressFunds;

use super::provider::PlatformPaymentAddressProvider;

/// Platform address wallet providing DIP-17 platform payment address functionality.
#[derive(Clone)]
pub struct PlatformAddressWallet {
    pub(crate) sdk: dash_sdk::Sdk,
    pub(crate) wallet: Arc<RwLock<Wallet>>,
    pub(crate) wallet_info: Arc<RwLock<ManagedWalletInfo>>,
    pub(crate) network: Network,
    /// Cached platform address balances from the last sync.
    balances: Arc<RwLock<BTreeMap<PlatformAddress, Credits>>>,
}

impl PlatformAddressWallet {
    /// Create a new PlatformAddressWallet.
    pub(crate) fn new(
        sdk: dash_sdk::Sdk,
        wallet: Arc<RwLock<Wallet>>,
        wallet_info: Arc<RwLock<ManagedWalletInfo>>,
        network: Network,
    ) -> Self {
        Self {
            sdk,
            wallet,
            wallet_info,
            network,
            balances: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }

    /// Get the cached network (sync, no lock needed).
    pub fn network(&self) -> Network {
        self.network
    }

    /// Sync platform address balances from Platform.
    ///
    /// Uses the SDK's privacy-preserving trunk/branch address synchronization
    /// with DIP-17 address discovery via gap limit scanning.
    pub async fn sync_balances(&self) -> Result<AddressSyncResult, PlatformWalletError> {
        // Build the address provider from the wallet.
        let mut provider =
            PlatformPaymentAddressProvider::from_wallet(self.wallet.clone(), self.network).map_err(
                |e| {
                    PlatformWalletError::AddressSync(format!(
                        "Failed to create address provider: {}",
                        e
                    ))
                },
            )?;

        let result = self
            .sdk
            .sync_address_balances(&mut provider, None, None)
            .await?;

        // Update cached balances from the sync results.
        let mut balances = self.balances.write().await;
        balances.clear();
        for ((_, key), funds) in &result.found {
            match PlatformAddress::from_bytes(key) {
                Ok(platform_addr) => {
                    balances.insert(platform_addr, funds.balance);
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to parse PlatformAddress from sync result key: {}",
                        e
                    );
                }
            }
        }

        Ok(result)
    }

    /// Transfer credits between platform addresses.
    ///
    /// Broadcasts an address funds transfer state transition. The fee is deducted
    /// from the first input address by default.
    pub async fn transfer(
        &self,
        inputs: BTreeMap<PlatformAddress, Credits>,
        outputs: BTreeMap<PlatformAddress, Credits>,
    ) -> Result<(), PlatformWalletError> {
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
        let mut balances = self.balances.write().await;
        for (addr, maybe_info) in address_infos.iter() {
            match maybe_info {
                Some(info) => {
                    balances.insert(*addr, info.balance);
                }
                None => {
                    balances.remove(addr);
                }
            }
        }

        Ok(())
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
    ) -> Result<(), PlatformWalletError> {
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
        let mut balances = self.balances.write().await;
        for (addr, maybe_info) in address_infos.iter() {
            match maybe_info {
                Some(info) => {
                    balances.insert(*addr, info.balance);
                }
                None => {
                    balances.remove(addr);
                }
            }
        }

        Ok(())
    }

    /// Get all platform addresses with their cached balances.
    ///
    /// Returns the balances from the last call to [`sync_balances`](Self::sync_balances),
    /// [`transfer`](Self::transfer), or [`withdraw`](Self::withdraw).
    pub async fn addresses_with_balances(&self) -> Vec<(PlatformAddress, Credits)> {
        let balances = self.balances.read().await;
        balances.iter().map(|(addr, &bal)| (*addr, bal)).collect()
    }

    /// Get total platform credits across all addresses.
    ///
    /// Returns the sum of all cached balances.
    pub async fn total_credits(&self) -> Credits {
        let balances = self.balances.read().await;
        balances.values().sum()
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

        // Step 1: find the derivation path (only needs wallet_info lock)
        let derivation_path = {
            let wallet_info = self.wallet_info.blocking_read();
            let mut found_path = None;
            for account in wallet_info.accounts.platform_payment_accounts.values() {
                for addr_info in account.addresses.addresses.values() {
                    let Ok(pool_addr) =
                        PlatformP2PKHAddress::from_address(&addr_info.address)
                    else {
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
            found_path
        }; // wallet_info lock dropped here

        let path = derivation_path.ok_or_else(|| {
            ProtocolError::Generic(format!(
                "Platform address {:?} not found in wallet",
                platform_address
            ))
        })?;

        // Step 2: derive the private key (only needs wallet lock)
        let wallet = self.wallet.blocking_read();
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

        let signature =
            dashcore::signer::sign(data, private_key_bytes.as_ref())
                .map_err(|e| ProtocolError::Generic(format!("Failed to sign: {}", e)))?;

        Ok(BinaryData::new(signature.to_vec()))
    }

    fn sign_create_witness(
        &self,
        platform_address: &PlatformAddress,
        data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        let private_key_bytes = self.find_private_key_for_platform_address(platform_address)?;

        let signature =
            dashcore::signer::sign(data, private_key_bytes.as_ref())
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
            .field("network", &self.network)
            .finish()
    }
}
