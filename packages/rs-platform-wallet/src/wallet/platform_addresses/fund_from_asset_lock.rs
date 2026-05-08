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

        // Get the cached key source from the unified provider for gap
        // limit maintenance.
        let key_source = {
            let guard = self.provider.read().await;
            guard
                .as_ref()
                .and_then(|p| p.key_source(&self.wallet_id, account_index))
        };

        // Update balances in the ManagedPlatformAccount.
        let mut wm = self.wallet_manager.write().await;
        let mut cs = PlatformAddressChangeSet::default();
        if let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) {
            if let Some(account) = info
                .core_wallet
                .platform_payment_managed_account_at_index_mut(account_index)
            {
                for (addr, maybe_info) in address_infos.iter() {
                    let PlatformAddress::P2pkh(hash) = addr else {
                        continue;
                    };
                    let p2pkh = PlatformP2PKHAddress::new(*hash);
                    let funds = match maybe_info {
                        Some(ai) => dash_sdk::platform::address_sync::AddressFunds {
                            balance: ai.balance,
                            nonce: ai.nonce,
                        },
                        None => dash_sdk::platform::address_sync::AddressFunds {
                            balance: 0,
                            nonce: 0,
                        },
                    };
                    account.set_address_credit_balance(p2pkh, funds.balance, key_source.as_ref());
                    let address_index = account
                        .addresses
                        .addresses
                        .iter()
                        .find_map(|(&idx, info)| {
                            PlatformP2PKHAddress::from_address(&info.address)
                                .ok()
                                .filter(|found| *found == p2pkh)
                                .map(|_| idx)
                        })
                        .unwrap_or(0);
                    cs.addresses.push(crate::PlatformAddressBalanceEntry {
                        wallet_id: self.wallet_id,
                        account_index,
                        address_index,
                        address: p2pkh,
                        funds,
                    });
                }
            }
        }

        Ok(cs)
    }
}
