mod v0;

pub use v0::*;

use crate::state_transition::shield_from_identity_transition::ShieldFromIdentityTransition;
#[cfg(feature = "state-transition-signing")]
use crate::{
    identity::{signer::Signer, Identity, IdentityPublicKey},
    prelude::{IdentityNonce, UserFeeIncrease},
    shielded::SerializedAction,
    state_transition::{
        shield_from_identity_transition::v0::ShieldFromIdentityTransitionV0, StateTransition,
    },
    ProtocolError,
};
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

impl ShieldFromIdentityTransitionMethodsV0 for ShieldFromIdentityTransition {
    #[cfg(feature = "state-transition-signing")]
    async fn try_from_bundle_with_identity_signer<S: Signer<IdentityPublicKey>>(
        identity: &Identity,
        amount: u64,
        actions: Vec<SerializedAction>,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        signing_transfer_key_to_use: Option<&IdentityPublicKey>,
        nonce: IdentityNonce,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError> {
        match platform_version
            .dpp
            .state_transition_serialization_versions
            .shield_from_identity_state_transition
            .default_current_version
        {
            0 => {
                ShieldFromIdentityTransitionV0::try_from_bundle_with_identity_signer(
                    identity,
                    amount,
                    actions,
                    anchor,
                    proof,
                    binding_signature,
                    user_fee_increase,
                    signer,
                    signing_transfer_key_to_use,
                    nonce,
                    platform_version,
                )
                .await
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "ShieldFromIdentityTransition::try_from_bundle_with_identity_signer"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
