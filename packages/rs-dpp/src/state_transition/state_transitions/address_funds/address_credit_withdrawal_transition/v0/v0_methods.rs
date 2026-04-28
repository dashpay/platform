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
use crate::state_transition::first_consensus_error_as_protocol_error;
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
    async fn try_from_inputs_with_signer<S: Signer<PlatformAddress>>(
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

        // Pre-signing structure check: validate everything except the witness
        // count, so structural errors fail fast before performing any async
        // signer work.
        let pre_validation_result = address_credit_withdrawal_transition
            .validate_structure_without_input_witnesses(platform_version);
        if let Some(error) = first_consensus_error_as_protocol_error(pre_validation_result) {
            return Err(error);
        }

        let state_transition: StateTransition = address_credit_withdrawal_transition.clone().into();

        let signable_bytes = state_transition.signable_bytes()?;

        let mut input_witnesses: Vec<AddressWitness> = Vec::with_capacity(inputs.len());
        for address in inputs.keys() {
            input_witnesses.push(signer.sign_create_witness(address, &signable_bytes).await?);
        }
        address_credit_withdrawal_transition.input_witnesses = input_witnesses;

        // After signing, only the witness count needs (re-)validation; the rest
        // of the structure was already verified above.
        let validation_result =
            address_credit_withdrawal_transition.validate_input_witnesses_count();
        if let Some(error) = first_consensus_error_as_protocol_error(validation_result) {
            return Err(error);
        }

        tracing::debug!("try_from_inputs_with_signer: Successfully created transition");
        Ok(address_credit_withdrawal_transition.into())
    }
}
