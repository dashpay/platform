use crate::wallet::PlatformAddressWallet;
use crate::{PlatformAddressChangeSet, PlatformWalletError};
use dash_sdk::platform::transition::top_up_address::TopUpAddress;
use dashcore::PrivateKey;
use dpp::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
use dpp::fee::Credits;
use dpp::identity::signer::Signer;
use dpp::prelude::AssetLockProof;
use key_wallet::PlatformP2PKHAddress;
use std::collections::BTreeMap;

impl PlatformAddressWallet {
    /// Fund platform addresses from a Core L1 asset lock.
    ///
    /// Broadcasts a top-up-address state transition that converts locked Dash
    /// into platform credits on the specified addresses.
    ///
    /// # Arguments
    ///
    /// * `account_index` - Platform payment account index.
    /// * `addresses` - Platform addresses to fund (with current balances for nonce lookup).
    /// * `asset_lock_proof` - Proof of the asset lock transaction on Core chain.
    /// * `asset_lock_private_key` - Private key corresponding to the asset lock.
    /// * `fee_strategy` - How the fee should be deducted.
    /// * `address_signer` - Signs each previously-funded input address's
    ///   contribution. The wallet struct itself carries no key material.
    #[allow(clippy::too_many_arguments)]
    pub async fn fund_from_asset_lock<S: Signer<PlatformAddress> + Send + Sync>(
        &self,
        account_index: u32,
        addresses: BTreeMap<PlatformAddress, Option<Credits>>,
        asset_lock_proof: AssetLockProof,
        asset_lock_private_key: PrivateKey,
        fee_strategy: AddressFundsFeeStrategy,
        address_signer: &S,
    ) -> Result<PlatformAddressChangeSet, PlatformWalletError> {
        if addresses.is_empty() {
            return Err(PlatformWalletError::AddressOperation(
                "fund_from_asset_lock requires at least one address".to_string(),
            ));
        }

        // Exactly one address must have None balance (the funding recipient).
        let none_count = addresses.values().filter(|v| v.is_none()).count();
        if none_count != 1 {
            return Err(PlatformWalletError::AddressOperation(format!(
                "Exactly one address must have None balance (the funding recipient), found {}",
                none_count
            )));
        }

        // Verify all addresses belong to the specified account.
        {
            let wm = self.wallet_manager.read().await;
            let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
                PlatformWalletError::WalletNotFound(format!(
                    "Wallet {:?} not found in wallet manager",
                    hex::encode(self.wallet_id)
                ))
            })?;
            let account = info
                .core_wallet
                .platform_payment_managed_account_at_index(account_index)
                .ok_or_else(|| {
                    PlatformWalletError::AddressSync(format!(
                        "No platform payment account at index {}",
                        account_index
                    ))
                })?;
            for addr in addresses.keys() {
                let PlatformAddress::P2pkh(hash) = addr else {
                    return Err(PlatformWalletError::AddressOperation(
                        "Only P2PKH addresses are supported".to_string(),
                    ));
                };
                let p2pkh = PlatformP2PKHAddress::new(*hash);
                if !account.contains_platform_address(&p2pkh) {
                    return Err(PlatformWalletError::AddressNotFound(format!(
                        "Address {} does not belong to account index {}",
                        p2pkh, account_index
                    )));
                }
            }
        }

        let address_infos = addresses
            .top_up(
                &self.sdk,
                asset_lock_proof,
                asset_lock_private_key,
                fee_strategy,
                address_signer,
                None,
            )
            .await?;

        // Bookkeeping shared with the orchestrated
        // `fund_addresses_with_funding` path so the two flows can't
        // drift apart on the post-success balance-write shape.
        Ok(self
            .write_address_balances_changeset(account_index, &address_infos)
            .await)
    }
}
