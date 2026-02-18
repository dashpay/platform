#[cfg(feature = "state-transition-signing")]
use std::collections::BTreeMap;

#[cfg(feature = "state-transition-signing")]
use crate::address_funds::{AddressFundsFeeStrategy, AddressWitness, PlatformAddress};
#[cfg(feature = "state-transition-signing")]
use crate::fee::Credits;
#[cfg(feature = "state-transition-signing")]
use crate::identity::signer::Signer;
#[cfg(feature = "state-transition-signing")]
use crate::serialization::Signable;
use crate::state_transition::address_funds_transfer_transition::methods::AddressFundsTransferTransitionMethodsV0;
use crate::state_transition::address_funds_transfer_transition::v0::AddressFundsTransferTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::StateTransitionStructureValidation;
#[cfg(feature = "state-transition-signing")]
use crate::{
    prelude::{AddressNonce, UserFeeIncrease},
    state_transition::StateTransition,
    ProtocolError,
};
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

impl AddressFundsTransferTransitionMethodsV0 for AddressFundsTransferTransitionV0 {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_inputs_with_signer<S: Signer<PlatformAddress>>(
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        outputs: BTreeMap<PlatformAddress, Credits>,
        fee_strategy: AddressFundsFeeStrategy,
        signer: &S,
        user_fee_increase: UserFeeIncrease,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError> {
        tracing::debug!("try_from_inputs_with_signer: Started");
        tracing::debug!(
            input_count = inputs.len(),
            output_count = outputs.len(),
            "try_from_inputs_with_signer"
        );

        // Create the unsigned transition
        let mut address_funds_transition = AddressFundsTransferTransitionV0 {
            inputs: inputs.clone(),
            outputs,
            fee_strategy,
            user_fee_increase,
            input_witnesses: Vec::new(),
        };

        let state_transition: StateTransition = address_funds_transition.clone().into();

        let signable_bytes = state_transition.signable_bytes()?;

        address_funds_transition.input_witnesses = inputs
            .keys()
            .map(|address| signer.sign_create_witness(address, &signable_bytes))
            .collect::<Result<Vec<AddressWitness>, ProtocolError>>()?;

        // Validate the fully-constructed transition structure
        let validation_result = address_funds_transition.validate_structure(platform_version);
        if !validation_result.is_valid() {
            let first_error = validation_result.errors.into_iter().next().unwrap();
            return Err(ProtocolError::ConsensusError(Box::new(first_error)));
        }

        tracing::debug!("try_from_inputs_with_signer: Successfully created transition");
        Ok(address_funds_transition.into())
    }
}
