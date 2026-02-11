#[cfg(feature = "state-transition-signing")]
use crate::prelude::AssetLockProof;
#[cfg(feature = "state-transition-signing")]
use crate::serialization::Signable;
#[cfg(feature = "state-transition-signing")]
use crate::shielded::SerializedAction;
use crate::state_transition::shield_from_asset_lock_transition::methods::ShieldFromAssetLockTransitionMethodsV0;
use crate::state_transition::shield_from_asset_lock_transition::v0::ShieldFromAssetLockTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::{
    prelude::UserFeeIncrease,
    state_transition::StateTransition,
    ProtocolError,
};
#[cfg(feature = "state-transition-signing")]
use dashcore::signer;
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

impl ShieldFromAssetLockTransitionMethodsV0 for ShieldFromAssetLockTransitionV0 {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_asset_lock_with_bundle(
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &[u8],
        actions: Vec<SerializedAction>,
        flags: u8,
        value_balance: i64,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
        user_fee_increase: UserFeeIncrease,
        _platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError> {
        // Create the unsigned transition
        let mut transition = ShieldFromAssetLockTransitionV0 {
            asset_lock_proof,
            actions,
            flags,
            value_balance,
            anchor,
            proof,
            binding_signature,
            user_fee_increase,
            signature: Default::default(),
        };

        // Compute signable bytes (signature field is excluded from sig hash)
        let state_transition: StateTransition = transition.clone().into();
        let signable_bytes = state_transition.signable_bytes()?;

        // Sign with the asset lock private key (ECDSA)
        let signature = signer::sign(&signable_bytes, asset_lock_proof_private_key)?;
        transition.signature = signature.to_vec().into();

        Ok(transition.into())
    }
}
