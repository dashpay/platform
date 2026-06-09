mod v0;

pub use v0::*;

#[cfg(feature = "state-transition-signing")]
use crate::address_funds::PlatformAddress;
#[cfg(feature = "state-transition-signing")]
use crate::shielded::SerializedAction;
use crate::state_transition::identity_create_from_shielded_pool_transition::IdentityCreateFromShieldedPoolTransition;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
#[cfg(feature = "state-transition-signing")]
use crate::{
    state_transition::identity_create_from_shielded_pool_transition::v0::IdentityCreateFromShieldedPoolTransitionV0,
    state_transition::StateTransition, ProtocolError,
};
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

impl IdentityCreateFromShieldedPoolTransitionMethodsV0
    for IdentityCreateFromShieldedPoolTransition
{
    #[cfg(feature = "state-transition-signing")]
    fn try_from_bundle(
        public_keys: Vec<IdentityPublicKeyInCreation>,
        denomination: u64,
        send_to_address_on_creation_failure: PlatformAddress,
        actions: Vec<SerializedAction>,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError> {
        match platform_version
            .dpp
            .state_transition_serialization_versions
            .identity_create_from_shielded_pool_state_transition
            .default_current_version
        {
            0 => IdentityCreateFromShieldedPoolTransitionV0::try_from_bundle(
                public_keys,
                denomination,
                send_to_address_on_creation_failure,
                actions,
                anchor,
                proof,
                binding_signature,
                platform_version,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityCreateFromShieldedPoolTransition::try_from_bundle".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
