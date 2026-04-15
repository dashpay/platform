use dpp::address_funds::{AddressWitness, PlatformAddress};
use dpp::identity::signer::Signer;
use dpp::platform_value::BinaryData;
use dpp::ProtocolError;
use key_wallet::PlatformP2PKHAddress;
use zeroize::Zeroizing;

use crate::error::PlatformWalletError;
use crate::wallet::PlatformAddressWallet;

impl PlatformAddressWallet {
    /// Find the private key for a platform address by searching all platform
    /// payment accounts' address pools.
    ///
    /// Returns the raw private key bytes wrapped in [`Zeroizing`] so they are
    /// automatically wiped from memory when the value is dropped.
    pub(crate) async fn find_private_key_for_platform_address(
        &self,
        p2pkh: &PlatformP2PKHAddress,
    ) -> Result<Zeroizing<[u8; 32]>, PlatformWalletError> {
        let dashcore_addr = p2pkh.to_address(self.sdk.network);

        let wm = self.wallet_manager.read().await;
        let (wallet, info) = wm.get_wallet_and_info(&self.wallet_id).ok_or_else(|| {
            PlatformWalletError::WalletNotFound(format!(
                "Wallet {:?} not found in wallet manager",
                hex::encode(self.wallet_id)
            ))
        })?;

        // Search all platform payment accounts for the address.
        let mut found_path = None;
        for account in info.core_wallet.accounts.platform_payment_accounts.values() {
            if let Some(addr_info) = account.addresses.address_info(&dashcore_addr) {
                found_path = Some(addr_info.path.clone());
                break;
            }
        }

        let path =
            found_path.ok_or_else(|| PlatformWalletError::AddressNotFound(format!("{}", p2pkh)))?;

        let secret_key = wallet.derive_private_key(&path).map_err(|e| {
            PlatformWalletError::KeyDerivation(format!(
                "Failed to derive private key for {}: {}",
                p2pkh, e
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
        let PlatformAddress::P2pkh(hash) = platform_address else {
            return Err(ProtocolError::Generic(
                "Only P2PKH Platform addresses are supported for signing".to_string(),
            ));
        };
        let p2pkh = PlatformP2PKHAddress::new(*hash);
        let handle = tokio::runtime::Handle::current();
        let private_key_bytes = tokio::task::block_in_place(|| {
            handle.block_on(self.find_private_key_for_platform_address(&p2pkh))
        })
        .map_err(|e| ProtocolError::Generic(e.to_string()))?;

        let signature = dashcore::signer::sign(data, private_key_bytes.as_ref())
            .map_err(|e| ProtocolError::Generic(format!("Failed to sign: {}", e)))?;

        Ok(BinaryData::new(signature.to_vec()))
    }

    fn sign_create_witness(
        &self,
        platform_address: &PlatformAddress,
        data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        let PlatformAddress::P2pkh(hash) = platform_address else {
            return Err(ProtocolError::Generic(
                "Only P2PKH Platform addresses are supported for signing".to_string(),
            ));
        };
        let p2pkh = PlatformP2PKHAddress::new(*hash);
        let handle = tokio::runtime::Handle::current();
        let private_key_bytes = tokio::task::block_in_place(|| {
            handle.block_on(self.find_private_key_for_platform_address(&p2pkh))
        })
        .map_err(|e| ProtocolError::Generic(e.to_string()))?;

        let signature = dashcore::signer::sign(data, private_key_bytes.as_ref())
            .map_err(|e| ProtocolError::Generic(format!("Failed to sign: {}", e)))?;

        Ok(AddressWitness::P2pkh {
            signature: BinaryData::new(signature.to_vec()),
        })
    }

    fn can_sign_with(&self, platform_address: &PlatformAddress) -> bool {
        let PlatformAddress::P2pkh(hash) = platform_address else {
            return false;
        };
        let p2pkh = PlatformP2PKHAddress::new(*hash);
        let Ok(handle) = tokio::runtime::Handle::try_current() else {
            return false;
        };
        tokio::task::block_in_place(|| {
            handle
                .block_on(self.find_private_key_for_platform_address(&p2pkh))
                .is_ok()
        })
    }
}
