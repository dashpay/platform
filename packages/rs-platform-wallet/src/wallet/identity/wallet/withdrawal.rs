//! Withdraw credits from an identity.

use dashcore::Address as DashAddress;
use dpp::identity::accessors::IdentitySettersV0;
use dpp::identity::Identity;
use dpp::identity::IdentityPublicKey;
use dpp::prelude::Identifier;

use dpp::identity::signer::Signer;

use dash_sdk::platform::transition::put_settings::PutSettings;
use dash_sdk::platform::transition::withdraw_from_identity::WithdrawFromIdentity;

use crate::error::PlatformWalletError;

use super::*;

// ---------------------------------------------------------------------------
// Withdrawal
// ---------------------------------------------------------------------------

impl IdentityWallet {
    /// Withdraw credits from an identity to a Dash address.
    ///
    /// Submits an `IdentityCreditWithdrawalTransition` to Platform that moves
    /// the specified amount (in platform credits) from the identity back to
    /// a Core chain address.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The identifier of the identity to withdraw from.
    /// * `amount` - Amount of credits to withdraw.
    /// * `to_address` - The Dash P2PKH address to receive the withdrawal.
    pub async fn withdraw_credits(
        &self,
        identity_id: &Identifier,
        amount: u64,
        to_address: &DashAddress,
        settings: Option<PutSettings>,
    ) -> Result<(), PlatformWalletError> {
        // Retrieve the identity and its HD index from the manager.
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
                .cloned()
                .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;
            let index = manager
                .identity_index(identity_id)
                .ok_or(PlatformWalletError::IdentityIndexNotSet(*identity_id))?;
            (identity, index)
        };

        let signer = self.signer_for_identity(identity_index);

        let new_balance = identity
            .withdraw(
                &self.sdk,
                Some(to_address.clone()),
                amount,
                None, // core_fee_per_byte
                None, // signing_withdrawal_key_to_use
                signer,
                settings,
            )
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to withdraw credits: {}",
                    e
                ))
            })?;

        // Update the identity's balance in the local manager.
        {
            let mut wm = self.wallet_manager.write().await;
            let info_guard = wm.get_wallet_info_mut(&self.wallet_id).ok_or_else(|| {
                crate::error::PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            if let Some(identity) = info_guard.identity_manager.identity_mut(identity_id) {
                identity.set_balance(new_balance);
            }
        }

        Ok(())
    }

    /// Withdraw credits using an externally-provided identity and signer.
    ///
    /// Unlike [`withdraw_credits`](Self::withdraw_credits), this method does
    /// **not** look up the identity in the internal `IdentityManager`. Instead,
    /// the caller supplies the `Identity` object and a `Signer` implementation
    /// directly. This is useful when the caller manages identities outside of
    /// the platform-wallet `IdentityManager` (e.g. evo-tool's
    /// `QualifiedIdentity`).
    ///
    /// Returns the remaining credit balance after the withdrawal.
    #[allow(clippy::too_many_arguments)]
    pub async fn withdraw_credits_with_signer<S: Signer<IdentityPublicKey> + Send>(
        &self,
        identity: &Identity,
        to_address: Option<DashAddress>,
        amount: u64,
        signing_withdrawal_key_to_use: Option<&IdentityPublicKey>,
        signer: S,
        settings: Option<PutSettings>,
    ) -> Result<u64, dash_sdk::Error> {
        identity
            .withdraw(
                &self.sdk,
                to_address,
                amount,
                Some(1), // core_fee_per_byte
                signing_withdrawal_key_to_use,
                signer,
                settings,
            )
            .await
    }
}
