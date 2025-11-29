mod v0;

#[cfg(feature = "state-transition-signing")]
use std::collections::BTreeMap;
pub use v0::*;

#[cfg(feature = "state-transition-signing")]
use crate::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
#[cfg(feature = "state-transition-signing")]
use crate::fee::Credits;
#[cfg(feature = "state-transition-signing")]
use crate::identity::signer::Signer;
use crate::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition;
#[cfg(feature = "state-transition-signing")]
use crate::{
    prelude::{KeyOfTypeNonce, UserFeeIncrease},
    state_transition::{
        address_funds_transfer_transition::v0::AddressFundsTransferTransitionV0, StateTransition,
    },
    ProtocolError,
};
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

impl AddressFundsTransferTransitionMethodsV0 for AddressFundsTransferTransition {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_inputs_with_signer<S: Signer<PlatformAddress>>(
        inputs: BTreeMap<PlatformAddress, (KeyOfTypeNonce, Credits)>,
        outputs: BTreeMap<PlatformAddress, Credits>,
        fee_strategy: AddressFundsFeeStrategy,
        signer: &S,
        user_fee_increase: UserFeeIncrease,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError> {
        match platform_version
            .dpp
            .state_transition_conversion_versions
            .address_funds_to_address_funds_transfer_transition
        {
            0 => Ok(
                AddressFundsTransferTransitionV0::try_from_inputs_with_signer::<S>(
                    inputs,
                    outputs,
                    fee_strategy,
                    signer,
                    user_fee_increase,
                    platform_version,
                )?,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "AddressFundsTransferTransition::try_from_inputs_with_signer".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
