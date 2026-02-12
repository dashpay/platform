#[cfg(feature = "state-transition-signing")]
use crate::shielded::SerializedAction;
use crate::state_transition::StateTransitionType;
#[cfg(feature = "state-transition-signing")]
use crate::{prelude::UserFeeIncrease, state_transition::StateTransition, ProtocolError};
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

pub trait ShieldedTransferTransitionMethodsV0 {
    #[cfg(feature = "state-transition-signing")]
    #[allow(clippy::too_many_arguments)]
    fn try_from_bundle(
        actions: Vec<SerializedAction>,
        flags: u8,
        value_balance: u64,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
        user_fee_increase: UserFeeIncrease,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError>;

    /// Get State Transition Type
    fn get_type() -> StateTransitionType {
        StateTransitionType::ShieldedTransfer
    }
}
