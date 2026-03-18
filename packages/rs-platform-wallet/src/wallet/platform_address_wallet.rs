//! Platform address wallet for DIP-17 platform payment addresses.

use std::sync::Arc;

use dpp::address_funds::{AddressWitness, PlatformAddress};
use dpp::identity::signer::Signer;
use dpp::platform_value::BinaryData;
use dpp::ProtocolError;
use key_wallet::PlatformP2PKHAddress;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use key_wallet::Network;
use tokio::sync::RwLock;

/// Platform address wallet providing DIP-17 platform payment address functionality.
#[derive(Clone)]
pub struct PlatformAddressWallet {
    pub(crate) sdk: dash_sdk::Sdk,
    pub(crate) wallet: Arc<RwLock<Wallet>>,
    pub(crate) wallet_info: Arc<RwLock<ManagedWalletInfo>>,
    pub(crate) network: Network,
}

impl PlatformAddressWallet {
    /// Get the cached network (sync, no lock needed).
    pub fn network(&self) -> Network {
        self.network
    }

    /// Find the derivation path for a platform address by searching all platform
    /// payment accounts in the wallet info.
    ///
    /// Returns the full derivation path to the matching address, or an error if
    /// the address is not found.
    fn find_private_key_for_platform_address(
        &self,
        platform_address: &PlatformAddress,
    ) -> Result<dashcore::secp256k1::SecretKey, ProtocolError> {
        let PlatformAddress::P2pkh(hash) = platform_address else {
            return Err(ProtocolError::Generic(
                "Only P2PKH Platform addresses are currently supported for signing".to_string(),
            ));
        };

        let target = PlatformP2PKHAddress::new(*hash);

        let wallet_info = self.wallet_info.blocking_read();
        let wallet = self.wallet.blocking_read();

        // Search through all platform payment accounts for a matching address
        for account in wallet_info.accounts.platform_payment_accounts.values() {
            for addr_info in account.addresses.addresses.values() {
                let Ok(pool_addr) = PlatformP2PKHAddress::from_address(&addr_info.address) else {
                    continue;
                };
                if pool_addr == target {
                    // Found the matching address — derive the private key
                    let secret_key = wallet
                        .derive_private_key(&addr_info.path)
                        .map_err(|e| {
                            ProtocolError::Generic(format!(
                                "Failed to derive private key for platform address: {}",
                                e
                            ))
                        })?;
                    return Ok(secret_key);
                }
            }
        }

        Err(ProtocolError::Generic(format!(
            "Platform address {:?} not found in wallet",
            platform_address
        )))
    }
}

impl Signer<PlatformAddress> for PlatformAddressWallet {
    fn sign(
        &self,
        platform_address: &PlatformAddress,
        data: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        let secret_key = self.find_private_key_for_platform_address(platform_address)?;

        let signature =
            dashcore::signer::sign(data, secret_key.as_ref())
                .map_err(|e| ProtocolError::Generic(format!("Failed to sign: {}", e)))?;

        Ok(BinaryData::new(signature.to_vec()))
    }

    fn sign_create_witness(
        &self,
        platform_address: &PlatformAddress,
        data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        let secret_key = self.find_private_key_for_platform_address(platform_address)?;

        let signature =
            dashcore::signer::sign(data, secret_key.as_ref())
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
