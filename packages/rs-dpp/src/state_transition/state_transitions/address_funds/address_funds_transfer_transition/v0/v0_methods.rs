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
use crate::state_transition::{
    address_funds_constructor_dispatch_error, consensus_errors_as_protocol_error,
    verify_address_witnesses, StateTransitionType,
};
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
    async fn try_from_inputs_with_signer<S: Signer<PlatformAddress>>(
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
            inputs,
            outputs,
            fee_strategy,
            user_fee_increase,
            input_witnesses: Vec::new(),
        };

        if let Some(error) = address_funds_constructor_dispatch_error(
            StateTransitionType::AddressFundsTransfer,
            platform_version,
        ) {
            return Err(error);
        }

        // Pre-signing structure check: validate everything except the witness
        // count, so structural errors fail fast before performing any async
        // signer work.
        //
        // LOCKSTEP: this call is hard-coded to the v0 basic-structure check.
        // If a future v1 basic-structure is introduced for this transition,
        // both the drive-abci server dispatcher AND this SDK constructor must
        // be updated together (e.g. by routing through a versioned
        // `validate_basic_structure` wrapper as IdentityUpdate does).
        let pre_validation_result =
            address_funds_transition.validate_structure_without_input_witnesses(platform_version);
        if let Some(error) = consensus_errors_as_protocol_error(pre_validation_result) {
            return Err(error);
        }

        let state_transition: StateTransition = address_funds_transition.clone().into();

        let signable_bytes = state_transition.signable_bytes()?;

        let mut input_witnesses: Vec<AddressWitness> =
            Vec::with_capacity(address_funds_transition.inputs.len());
        for address in address_funds_transition.inputs.keys() {
            input_witnesses.push(signer.sign_create_witness(address, &signable_bytes).await?);
        }
        verify_address_witnesses(
            address_funds_transition.inputs.keys(),
            &input_witnesses,
            &signable_bytes,
        )?;
        address_funds_transition.input_witnesses = input_witnesses;

        // After signing, only the witness count needs (re-)validation; the rest
        // of the structure was already verified above.
        let validation_result = address_funds_transition.validate_input_witnesses_count();
        if let Some(error) = consensus_errors_as_protocol_error(validation_result) {
            return Err(error);
        }

        tracing::debug!("try_from_inputs_with_signer: Successfully created transition");
        Ok(address_funds_transition.into())
    }
}
