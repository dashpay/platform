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
                .map(|m| m.identity.clone())
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

        // Update the identity's balance in the local manager and
        // queue the snapshot so the new balance survives relaunch.
        // See the comment on `top_up` for rationale on driving the
        // persister directly from the call site instead of through
        // a dedicated `ManagedIdentity::set_balance` method.
        {
            let mut wm = self.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&self.wallet_id).ok_or_else(|| {
                crate::error::PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            if let Some(managed) = info.identity_manager.managed_identity_mut(identity_id) {
                managed.identity.set_balance(new_balance);
                if let Err(e) = self.persister.store(managed.snapshot_changeset().into()) {
                    tracing::error!(
                        identity = %identity_id,
                        error = %e,
                        "Failed to persist identity balance update after top_up_from_addresses"
                    );
                }
            }
        }

        Ok(new_balance)
    }
}
