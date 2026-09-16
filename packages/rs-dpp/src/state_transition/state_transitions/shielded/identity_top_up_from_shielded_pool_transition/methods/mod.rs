mod v0;

pub use v0::*;

#[cfg(feature = "state-transition-signing")]
use crate::shielded::SerializedAction;
use crate::state_transition::identity_top_up_from_shielded_pool_transition::IdentityTopUpFromShieldedPoolTransition;
#[cfg(feature = "state-transition-signing")]
use crate::{
    state_transition::{
        identity_top_up_from_shielded_pool_transition::v0::IdentityTopUpFromShieldedPoolTransitionV0,
        StateTransition,
    },
    ProtocolError,
};
#[cfg(feature = "state-transition-signing")]
use platform_value::Identifier;
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

impl IdentityTopUpFromShieldedPoolTransitionMethodsV0 for IdentityTopUpFromShieldedPoolTransition {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_bundle(
        identity_id: Identifier,
        actions: Vec<SerializedAction>,
        top_up_amount: u64,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError> {
        match platform_version
            .dpp
            .state_transition_serialization_versions
            .identity_top_up_from_shielded_pool_state_transition
            .default_current_version
        {
            0 => IdentityTopUpFromShieldedPoolTransitionV0::try_from_bundle(
                identity_id,
                actions,
                top_up_amount,
                anchor,
                proof,
                binding_signature,
                platform_version,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityTopUpFromShieldedPoolTransition::try_from_bundle".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
