use super::broadcast::BroadcastStateTransition;
use super::put_settings::PutSettings;
use super::validation::ensure_valid_state_transition_structure;
use crate::{Error, Sdk};
use dpp::prelude::AssetLockProof;
use dpp::shielded::OrchardBundleParams;
use dpp::state_transition::shield_from_asset_lock_transition::methods::ShieldFromAssetLockTransitionMethodsV0;
use dpp::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;
use std::future::Future;
use std::pin::Pin;

/// Helper trait to shield funds from an L1 asset lock into the shielded pool.
pub trait ShieldFromAssetLock {
    /// Shield funds from an L1 asset lock into the shielded pool.
    /// The asset lock proof proves ownership of L1 funds, and the ECDSA signature
    /// binds those funds to this specific Orchard bundle.
    fn shield_from_asset_lock<'a>(
        &'a self,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &'a [u8],
        bundle: OrchardBundleParams,
        value_balance: u64,
        settings: Option<PutSettings>,
    ) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send + 'a>>;
}

impl ShieldFromAssetLock for Sdk {
    fn shield_from_asset_lock<'a>(
        &'a self,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &'a [u8],
        bundle: OrchardBundleParams,
        value_balance: u64,
        settings: Option<PutSettings>,
    ) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send + 'a>> {
        Box::pin(async move {
            let OrchardBundleParams {
                actions,
                anchor,
                proof,
                binding_signature,
            } = bundle;

            let state_transition = ShieldFromAssetLockTransition::try_from_asset_lock_with_bundle(
                asset_lock_proof,
                asset_lock_proof_private_key,
                actions,
                value_balance,
                anchor,
                proof,
                binding_signature,
                self.version(),
            )?;
            ensure_valid_state_transition_structure(&state_transition, self.version())?;

            state_transition.broadcast(self, settings).await?;
            Ok(())
        })
    }
}
