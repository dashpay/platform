mod v0;

pub use v0::*;

#[cfg(feature = "state-transition-signing")]
use crate::shielded::SerializedAction;
use crate::state_transition::shielded_transfer_transition::ShieldedTransferTransition;
#[cfg(feature = "state-transition-signing")]
use crate::{
    state_transition::{
        shielded_transfer_transition::v0::ShieldedTransferTransitionV0, StateTransition,
    },
    ProtocolError,
};
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

impl ShieldedTransferTransitionMethodsV0 for ShieldedTransferTransition {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_bundle(
        actions: Vec<SerializedAction>,
        value_balance: u64,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError> {
        match platform_version
            .dpp
            .state_transition_serialization_versions
            .shielded_transfer_state_transition
            .default_current_version
        {
            0 => ShieldedTransferTransitionV0::try_from_bundle(
                actions,
                value_balance,
                anchor,
                proof,
                binding_signature,
                platform_version,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "ShieldedTransferTransition::try_from_bundle".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
