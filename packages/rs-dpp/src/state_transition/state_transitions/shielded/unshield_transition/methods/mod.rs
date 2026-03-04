mod v0;

pub use v0::*;

#[cfg(feature = "state-transition-signing")]
use crate::address_funds::PlatformAddress;
#[cfg(feature = "state-transition-signing")]
use crate::shielded::SerializedAction;
use crate::state_transition::unshield_transition::UnshieldTransition;
#[cfg(feature = "state-transition-signing")]
use crate::{
    state_transition::{unshield_transition::v0::UnshieldTransitionV0, StateTransition},
    ProtocolError,
};
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

impl UnshieldTransitionMethodsV0 for UnshieldTransition {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_bundle(
        output_address: PlatformAddress,
        amount: u64,
        actions: Vec<SerializedAction>,
        value_balance: i64,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError> {
        match platform_version
            .dpp
            .state_transition_serialization_versions
            .unshield_state_transition
            .default_current_version
        {
            0 => UnshieldTransitionV0::try_from_bundle(
                output_address,
                amount,
                actions,
                value_balance,
                anchor,
                proof,
                binding_signature,
                platform_version,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "UnshieldTransition::try_from_bundle".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
