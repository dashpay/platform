//! Top up an identity's credit balance.

use std::time::Duration;

use dpp::identity::accessors::IdentitySettersV0;
use dpp::prelude::Identifier;

use dash_sdk::platform::transition::put_settings::PutSettings;
use dash_sdk::platform::transition::top_up_identity::TopUpIdentity;

use crate::error::PlatformWalletError;

use crate::wallet::identity::funding::TopUpFundingMethod;

use super::*;

// ---------------------------------------------------------------------------
// Top-up
// ---------------------------------------------------------------------------

impl IdentityWallet {
    /// Top up an existing identity's credit balance.
    ///
    /// Convenience wrapper that uses `FundWithWallet` funding. For other
    /// funding methods, use [`top_up_identity_with_funding`](Self::top_up_identity_with_funding).
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The identifier of the identity to top up.
    /// * `topup_index` - An incrementing index distinguishing successive
    ///   top-ups for the same identity.
    /// * `amount_duffs` - Amount of Dash (in duffs) to add.
    pub async fn top_up_identity(
        &self,
        identity_id: &Identifier,
        topup_index: u32,
        amount_duffs: u64,
        settings: Option<PutSettings>,
    ) -> Result<(), PlatformWalletError> {
        self.top_up_identity_with_funding(
            identity_id,
            TopUpFundingMethod::FundWithWallet { amount_duffs },
            topup_index,
            settings,
        )
        .await
    }

    /// Top up an existing identity's credit balance with a specified funding method.
    ///
    /// # Funding methods
    ///
    /// * `UseAssetLock` - Use a pre-existing proof and private key directly.
    /// * `FundWithWallet` - Build an asset lock from wallet UTXOs (default).
    ///
    /// # IS -> CL fallback
    ///
    /// See [`register_identity_with_funding`](Self::register_identity_with_funding)
    /// for details on the IS -> CL fallback strategy.
    pub async fn top_up_identity_with_funding(
        &self,
        identity_id: &Identifier,
        funding: TopUpFundingMethod,
        // TODO(platform-wallet): route `topup_index` through the
        // derivation path for the top-up asset lock. Currently
        // unused; the function derives from `identity_index`
        // alone.
        _topup_index: u32,
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

        // Step 1: Obtain the asset lock proof and private key.
        let (asset_lock_proof, asset_lock_private_key) = match funding {
            TopUpFundingMethod::UseAssetLock { proof, private_key } => (proof, private_key),
            TopUpFundingMethod::FundWithWallet { amount_duffs } => {
                use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
                let (proof, key, _out_point) = self
                    .asset_locks
                    .create_funded_asset_lock_proof(
                        amount_duffs,
                        0,
                        AssetLockFundingType::IdentityTopUp,
                        identity_index,
                    )
                    .await?;
                (proof, key)
            }
        };

        // Extract the outpoint before consuming the proof, in case we need to
        // build a ChainLock proof for recovery.
        let proof_out_point = Self::out_point_from_proof(&asset_lock_proof);

        // Step 2: Submit the top-up state transition.
        let user_fee_increase = settings.and_then(|s| s.user_fee_increase);
        let new_balance = match identity
            .top_up_identity(
                &self.sdk,
                asset_lock_proof,
                &asset_lock_private_key,
                user_fee_increase,
                settings,
            )
            .await
        {
            Ok(balance) => balance,
            Err(e) if crate::error::is_instant_lock_proof_invalid(&e) => {
                // IS-lock proof was rejected — try to upgrade to ChainLock.
                if let Some(out_point) = proof_out_point {
                    tracing::warn!(
                        "IS-lock proof rejected for identity top-up (tx {}), \
                         retrying with ChainLock proof",
                        out_point.txid
                    );
                    let chain_proof = self
                        .asset_locks
                        .upgrade_to_chain_lock_proof(&out_point, Duration::from_secs(180))
                        .await?;
                    identity
                        .top_up_identity(
                            &self.sdk,
                            chain_proof,
                            &asset_lock_private_key,
                            user_fee_increase,
                            settings,
                        )
                        .await
                        .map_err(|e| {
                            PlatformWalletError::InvalidIdentityData(format!(
                                "Failed to top up identity (ChainLock retry): {}",
                                e
                            ))
                        })?
                } else {
                    return Err(PlatformWalletError::InvalidIdentityData(format!(
                        "Failed to top up identity: {}",
                        e
                    )));
                }
            }
            Err(e) => {
                return Err(PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to top up identity: {}",
                    e
                )));
            }
        };

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

        Ok(())
    }
}
