#[cfg(feature = "state-transition-signing")]
use crate::address_funds::AddressFundsFeeStrategy;
#[cfg(feature = "state-transition-signing")]
use crate::fee::Credits;
use crate::identity::signer::Signer;
#[cfg(feature = "state-transition-signing")]
use crate::identity::KeyOfType;
use crate::state_transition::StateTransitionType;
#[cfg(feature = "state-transition-signing")]
use crate::{
    prelude::{KeyOfTypeNonce, UserFeeIncrease},
    state_transition::StateTransition,
    ProtocolError,
};
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;
#[cfg(feature = "state-transition-signing")]
use std::collections::BTreeMap;

pub trait AddressFundsTransferTransitionMethodsV0 {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_inputs_with_signer<S: Signer<KeyOfType>>(
        inputs: BTreeMap<KeyOfType, (KeyOfTypeNonce, Credits)>,
        outputs: BTreeMap<KeyOfType, Credits>,
        fee_strategy: AddressFundsFeeStrategy,
        signer: &S,
        user_fee_increase: UserFeeIncrease,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError>;

    /// Get State Transition Type
    fn get_type() -> StateTransitionType {
        StateTransitionType::AddressFundsTransfer
    }
}
