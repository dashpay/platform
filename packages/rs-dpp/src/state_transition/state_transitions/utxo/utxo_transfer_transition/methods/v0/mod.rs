#[cfg(feature = "state-transition-signing")]
use crate::fee::Credits;
#[cfg(feature = "state-transition-signing")]
use crate::identity::KeyOfType;
use crate::state_transition::StateTransitionType;
#[cfg(feature = "state-transition-signing")]
use crate::{
    identity::signer::Signer, prelude::UserFeeIncrease, state_transition::StateTransition,
    ProtocolError,
};
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;
#[cfg(feature = "state-transition-signing")]
use std::collections::BTreeMap;

pub trait UTXOTransferTransitionMethodsV0 {
    #[cfg(feature = "state-transition-signing")]
    #[allow(clippy::too_many_arguments)]
    fn try_from_inputs_with_signer<S: Signer>(
        inputs: Vec<KeyOfTypeWithNonce>,
        outputs: BTreeMap<KeyOfType, Credits>,
        input_private_keys: Vec<&[u8]>,
        signer: &S,
        user_fee_increase: UserFeeIncrease,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError>;

    /// Get State Transition Type
    fn get_type() -> StateTransitionType {
        StateTransitionType::UTXOTransfer
    }
}
