use crate::wallet::PlatformAddressWallet;
use crate::{PlatformAddressChangeSet, PlatformWalletError};
use dash_sdk::platform::transition::top_up_address::TopUpAddress;
use dashcore::PrivateKey;
use dpp::address_funds::{AddressFundsFeeStrategyStep, PlatformAddress};
use dpp::fee::Credits;
use dpp::prelude::AssetLockProof;
use key_wallet::PlatformP2PKHAddress;
use std::collections::BTreeMap;

impl PlatformAddressWallet {
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

        // Update balances in the ManagedPlatformAccount.
        let mut wm = self.wallet_manager.write().await;
        let mut cs = PlatformAddressChangeSet::default();
        if let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) {
            if let Some(account) = info
                .core_wallet
                .first_platform_payment_managed_account_mut()
            {
                for (addr, maybe_info) in address_infos.iter() {
                    match maybe_info {
                        Some(ai) => {
                            if let PlatformAddress::P2pkh(hash) = addr {
                                let p2pkh = PlatformP2PKHAddress::new(*hash);
                                account.set_address_credit_balance(p2pkh, ai.balance, None);
                            }
                            cs.addresses.insert(*addr, ai.balance);
                        }
                        None => {
                            if let PlatformAddress::P2pkh(hash) = addr {
                                let p2pkh = PlatformP2PKHAddress::new(*hash);
                                account.set_address_credit_balance(p2pkh, 0, None);
                            }
                            cs.removed.insert(*addr);
                        }
                    }
                }
            }
        }

        Ok(cs)
    }
}
