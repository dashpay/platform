//! Transfer identity credits to platform addresses.

use std::collections::BTreeMap;

use dpp::identity::accessors::IdentitySettersV0;
use dpp::prelude::Identifier;

use dash_sdk::platform::transition::put_settings::PutSettings;
use dash_sdk::platform::transition::transfer_to_addresses::TransferToAddresses;

use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;

use crate::error::PlatformWalletError;

use super::*;

// ---------------------------------------------------------------------------
// Transfer credits to platform addresses
// ---------------------------------------------------------------------------

impl IdentityWallet {
    /// Transfer credits from an identity to multiple platform addresses.
    ///
    /// Uses the `TransferToAddresses` SDK trait.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The sending identity (must be owned by this wallet).
    /// * `recipient_addresses` - Map of platform addresses to credit amounts.
    pub async fn transfer_credits_to_addresses(
        &self,
        identity_id: &Identifier,
        recipient_addresses: BTreeMap<PlatformAddress, Credits>,
        settings: Option<PutSettings>,
    ) -> Result<Credits, PlatformWalletError> {
        let (identity, identity_index) = {
            let wm = self.wallet_manager.read().await;
            let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
                crate::error::PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            let manager = &info.identity_manager;
            let identity = manager
                .identity(identity_id)
                .map(|m| m.identity.clone())
                .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;
            let index = manager
                .identity_index(identity_id)
                .ok_or(PlatformWalletError::IdentityIndexNotSet(*identity_id))?;
            (identity, index)
        };

        let signer = self.signer_for_identity(identity_index);

        let (_address_infos, new_balance) = identity
            .transfer_credits_to_addresses(
                &self.sdk,
                recipient_addresses,
                None, // signing_transfer_key_to_use
                &signer,
                settings,
            )
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to transfer credits to addresses: {}",
                    e
                ))
            })?;

        // Update the sender's balance in the local manager and
        // queue the snapshot so the new balance survives relaunch.
        // See the comment on `top_up` for rationale on driving the
        // persister directly from the call site instead of through
        // a dedicated `ManagedIdentity::set_balance` method.
        {
            let mut wm = self.wallet_manager.write().await;
            let info_guard = wm.get_wallet_info_mut(&self.wallet_id).ok_or_else(|| {
                crate::error::PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            if let Some(managed) = info_guard
                .identity_manager
                .managed_identity_mut(identity_id)
            {
                managed.identity.set_balance(new_balance);
                if let Err(e) = self.persister.store(managed.snapshot_changeset().into()) {
                    tracing::error!(
                        identity = %identity_id,
                        error = %e,
                        "Failed to persist identity balance update after transfer_to_addresses"
                    );
                }
            }
        }

        Ok(new_balance)
    }
}
