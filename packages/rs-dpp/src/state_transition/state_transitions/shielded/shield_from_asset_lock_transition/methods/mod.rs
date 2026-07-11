mod v0;

pub use v0::*;

#[cfg(feature = "state-transition-signing")]
use crate::address_funds::PlatformAddress;
#[cfg(feature = "state-transition-signing")]
use crate::prelude::AssetLockProof;
#[cfg(feature = "state-transition-signing")]
use crate::shielded::SerializedAction;
use crate::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;
#[cfg(feature = "state-transition-signing")]
use crate::{
    state_transition::{
        shield_from_asset_lock_transition::v0::ShieldFromAssetLockTransitionV0, StateTransition,
    },
    ProtocolError,
};
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

impl ShieldFromAssetLockTransitionMethodsV0 for ShieldFromAssetLockTransition {
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
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError> {
        match platform_version
            .dpp
            .state_transition_serialization_versions
            .shield_from_asset_lock_state_transition
            .default_current_version
        {
            0 => ShieldFromAssetLockTransitionV0::try_from_asset_lock_with_bundle(
                asset_lock_proof,
                asset_lock_proof_private_key,
                actions,
                value_balance,
                anchor,
                proof,
                binding_signature,
                surplus_output,
                platform_version,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "ShieldFromAssetLockTransition::try_from_asset_lock_with_bundle"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
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
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError>
    where
        AS: ::key_wallet::signer::Signer,
    {
        match platform_version
            .dpp
            .state_transition_serialization_versions
            .shield_from_asset_lock_state_transition
            .default_current_version
        {
            0 => {
                ShieldFromAssetLockTransitionV0::try_from_asset_lock_with_bundle_and_signer(
                    asset_lock_proof,
                    asset_lock_proof_path,
                    asset_lock_signer,
                    actions,
                    value_balance,
                    anchor,
                    proof,
                    binding_signature,
                    surplus_output,
                    platform_version,
                )
                .await
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "ShieldFromAssetLockTransition::try_from_asset_lock_with_bundle_and_signer"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
