//! Transfer credits between identities.

use dpp::identity::accessors::IdentitySettersV0;
use dpp::identity::Identity;
use dpp::identity::IdentityPublicKey;
use dpp::prelude::Identifier;

use dpp::identity::signer::Signer;

use dash_sdk::platform::transition::put_settings::PutSettings;
use dash_sdk::platform::transition::transfer::TransferToIdentity;

use crate::error::PlatformWalletError;

use super::*;

// ---------------------------------------------------------------------------
// Credit transfer
// ---------------------------------------------------------------------------

impl IdentityWallet {
    /// Transfer credits from one identity to another.
    ///
    /// Submits an `IdentityCreditTransferTransition` to Platform that moves
    /// `amount` credits from `from_id` to `to_id`.
    ///
    /// # Arguments
    ///
    /// * `from_id` - The identifier of the sending identity (must be owned
    ///   by this wallet).
    /// * `to_id` - The identifier of the receiving identity.
    /// * `amount` - Amount of credits to transfer.
    pub async fn transfer_credits(
        &self,
        from_id: &Identifier,
        to_id: &Identifier,
        amount: u64,
        settings: Option<PutSettings>,
    ) -> Result<(), PlatformWalletError> {
        // Retrieve the sending identity and its HD index from the manager.
        let (identity, identity_index) = {
            let wm = self.wallet_manager.read().await;
            let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
                crate::error::PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            let manager = &info.identity_manager;
            let identity = manager
                .identity(from_id)
                .cloned()
                .ok_or(PlatformWalletError::IdentityNotFound(*from_id))?;
            let index = manager
                .identity_index(from_id)
                .ok_or(PlatformWalletError::IdentityIndexNotSet(*from_id))?;
            (identity, index)
        };

        let signer = self.signer_for_identity(identity_index);

        let (sender_balance, _receiver_balance) = identity
            .transfer_credits(
                &self.sdk, *to_id, amount, None, // signing_transfer_key_to_use
                signer, settings,
            )
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to transfer credits: {}",
                    e
                ))
            })?;

        // Update the sender's balance in the local manager.
        {
            let mut wm = self.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&self.wallet_id).ok_or_else(|| {
                crate::error::PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            if let Some(identity) = info.identity_manager.identity_mut(from_id) {
                identity.set_balance(sender_balance);
            }
        }

        Ok(())
    }

    /// Transfer credits using an externally-provided identity and signer.
    ///
    /// Unlike [`transfer_credits`](Self::transfer_credits), this method does
    /// **not** look up the identity in the internal `IdentityManager`. The
    /// caller supplies the `Identity` and a `Signer` directly.
    ///
    /// Returns `(sender_balance, receiver_balance)` after the transfer.
    pub async fn transfer_credits_with_signer<S: Signer<IdentityPublicKey> + Send>(
        &self,
        identity: &Identity,
        to_id: Identifier,
        amount: u64,
        signing_transfer_key_to_use: Option<&IdentityPublicKey>,
        signer: S,
        settings: Option<PutSettings>,
    ) -> Result<(u64, u64), dash_sdk::Error> {
        identity
            .transfer_credits(
                &self.sdk,
                to_id,
                amount,
                signing_transfer_key_to_use,
                signer,
                settings,
            )
            .await
    }
}
