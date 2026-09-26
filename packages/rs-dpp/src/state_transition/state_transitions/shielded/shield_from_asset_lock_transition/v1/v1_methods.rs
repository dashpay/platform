#[cfg(feature = "state-transition-signing")]
use crate::address_funds::PlatformAddress;
#[cfg(feature = "state-transition-signing")]
use crate::prelude::AssetLockProof;
#[cfg(feature = "state-transition-signing")]
use crate::serialization::Signable;
#[cfg(feature = "state-transition-signing")]
use crate::shielded::SerializedAction;
use crate::state_transition::shield_from_asset_lock_transition::methods::ShieldFromAssetLockTransitionMethodsV0;
use crate::state_transition::shield_from_asset_lock_transition::v1::ShieldFromAssetLockTransitionV1;
#[cfg(feature = "state-transition-signing")]
use crate::{state_transition::StateTransition, ProtocolError};
#[cfg(feature = "state-transition-signing")]
use dashcore::signer;
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

impl ShieldFromAssetLockTransitionMethodsV0 for ShieldFromAssetLockTransitionV1 {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_asset_lock_with_bundle(
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &[u8],
        actions: Vec<SerializedAction>,
        value_balance: u64,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
        surplus_output: Option<PlatformAddress>,
        _platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError> {
        let mut transition = ShieldFromAssetLockTransitionV1 {
            asset_lock_proof,
            actions,
            value_balance,
            anchor,
            proof,
            binding_signature,
            surplus_output,
            signature: Default::default(),
        };

        // The signable bytes carry the transition version and exclude the signature field.
        let state_transition: StateTransition = transition.clone().into();
        let signable_bytes = state_transition.signable_bytes()?;

        let signature = signer::sign(&signable_bytes, asset_lock_proof_private_key)?;
        transition.signature = signature.to_vec().into();

        Ok(transition.into())
    }

    #[cfg(all(feature = "state-transition-signing", feature = "core_key_wallet"))]
    async fn try_from_asset_lock_with_bundle_and_signer<AS>(
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_path: &::key_wallet::bip32::DerivationPath,
        asset_lock_signer: &AS,
        actions: Vec<SerializedAction>,
        value_balance: u64,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
        surplus_output: Option<PlatformAddress>,
        _platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError>
    where
        AS: ::key_wallet::signer::Signer,
    {
        let transition = ShieldFromAssetLockTransitionV1 {
            asset_lock_proof,
            actions,
            value_balance,
            anchor,
            proof,
            binding_signature,
            surplus_output,
            signature: Default::default(),
        };

        // `sign_with_core_signer` derives, signs and zeroises inside the external signer; the
        // host never sees the raw key.
        let mut state_transition: StateTransition = transition.into();
        state_transition
            .sign_with_core_signer(asset_lock_proof_path, asset_lock_signer)
            .await?;

        Ok(state_transition)
    }
}
