#[cfg(feature = "state-transition-signing")]
use std::collections::BTreeMap;

#[cfg(feature = "state-transition-signing")]
use crate::address_funds::{AddressFundsFeeStrategy, AddressWitness, PlatformAddress};
#[cfg(feature = "state-transition-signing")]
use crate::fee::Credits;
#[cfg(feature = "state-transition-signing")]
use crate::identity::core_script::CoreScript;
#[cfg(feature = "state-transition-signing")]
use crate::identity::signer::Signer;
#[cfg(feature = "state-transition-signing")]
use crate::serialization::Signable;
use crate::state_transition::address_credit_withdrawal_transition::methods::AddressCreditWithdrawalTransitionMethodsV0;
use crate::state_transition::address_credit_withdrawal_transition::v0::AddressCreditWithdrawalTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::StateTransitionStructureValidation;
#[cfg(feature = "state-transition-signing")]
use crate::withdrawal::Pooling;
#[cfg(feature = "state-transition-signing")]
use crate::{
    prelude::{AddressNonce, UserFeeIncrease},
    state_transition::StateTransition,
    ProtocolError,
};
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

impl AddressCreditWithdrawalTransitionMethodsV0 for AddressCreditWithdrawalTransitionV0 {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_inputs_with_signer<S: Signer<PlatformAddress>>(
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        output: Option<(PlatformAddress, Credits)>,
        fee_strategy: AddressFundsFeeStrategy,
        core_fee_per_byte: u32,
        pooling: Pooling,
        output_script: CoreScript,
        signer: &S,
        user_fee_increase: UserFeeIncrease,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError> {
        tracing::debug!("try_from_inputs_with_signer: Started");
        tracing::debug!(
            input_count = inputs.len(),
            has_output = output.is_some(),
            core_fee_per_byte = core_fee_per_byte,
            "try_from_inputs_with_signer"
        );

        // Create the unsigned transition
        let mut address_credit_withdrawal_transition = AddressCreditWithdrawalTransitionV0 {
            inputs: inputs.clone(),
            output,
            fee_strategy,
            core_fee_per_byte,
            pooling,
            output_script,
            user_fee_increase,
            input_witnesses: Vec::new(),
        };

        let state_transition: StateTransition = address_credit_withdrawal_transition.clone().into();

        let signable_bytes = state_transition.signable_bytes()?;

        address_credit_withdrawal_transition.input_witnesses = inputs
            .keys()
            .map(|address| signer.sign_create_witness(address, &signable_bytes))
            .collect::<Result<Vec<AddressWitness>, ProtocolError>>()?;

        // Validate the fully-constructed transition structure
        let validation_result =
            address_credit_withdrawal_transition.validate_structure(platform_version);
        if !validation_result.is_valid() {
            let first_error = validation_result.errors.into_iter().next().unwrap();
            return Err(ProtocolError::ConsensusError(Box::new(first_error)));
        }

        tracing::debug!("try_from_inputs_with_signer: Successfully created transition");
        Ok(address_credit_withdrawal_transition.into())
    }
}
