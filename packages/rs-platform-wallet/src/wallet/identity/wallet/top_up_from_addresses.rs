//! Top-up an identity from platform addresses.

use std::collections::BTreeMap;

use dpp::identity::accessors::IdentitySettersV0;
use dpp::prelude::Identifier;

use dash_sdk::platform::transition::put_settings::PutSettings;
use dash_sdk::platform::transition::top_up_identity_from_addresses::TopUpIdentityFromAddresses;

use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;

use crate::error::PlatformWalletError;
use crate::wallet::platform_addresses::PlatformAddressWallet;

use super::*;

// ---------------------------------------------------------------------------
// Top-up from platform addresses
// ---------------------------------------------------------------------------

impl IdentityWallet {
    /// Top up an identity by spending platform address balances.
    ///
    /// Uses the `TopUpIdentityFromAddresses` SDK trait. Address nonces are
    /// looked up automatically.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The identity to top up.
    /// * `inputs` - Map of platform addresses to credit amounts to spend.
    /// * `platform_address_wallet` - The platform address wallet (provides signing).
    pub async fn top_up_from_addresses(
        &self,
        identity_id: &Identifier,
        inputs: BTreeMap<PlatformAddress, Credits>,
        platform_address_wallet: &PlatformAddressWallet,
        settings: Option<PutSettings>,
    ) -> Result<Credits, PlatformWalletError> {
        let identity = {
            let wm = self.wallet_manager.read().await;
            let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
                crate::error::PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            let manager = &info.identity_manager;
            manager
                .identity(identity_id)
                .cloned()
                .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?
        };

        let (_address_infos, new_balance) = identity
            .top_up_from_addresses(&self.sdk, inputs, platform_address_wallet, settings)
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to top up identity from addresses: {}",
                    e
                ))
            })?;

        // Update the identity's balance in the local manager.
        {
            let mut wm = self.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&self.wallet_id).ok_or_else(|| {
                crate::error::PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            if let Some(identity) = info.identity_manager.identity_mut(identity_id) {
                identity.set_balance(new_balance);
            }
        }

        Ok(new_balance)
    }
}
