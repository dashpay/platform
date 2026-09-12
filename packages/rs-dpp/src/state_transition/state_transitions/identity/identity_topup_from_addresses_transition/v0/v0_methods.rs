// =====================================
// Ungated Imports
// =====================================
use crate::address_funds::PlatformAddress;
use crate::fee::Credits;
use crate::prelude::Identifier;
use crate::state_transition::identity_topup_from_addresses_transition::accessors::IdentityTopUpFromAddressesTransitionAccessorsV0;
use crate::state_transition::identity_topup_from_addresses_transition::methods::IdentityTopUpFromAddressesTransitionMethodsV0;
use crate::state_transition::identity_topup_from_addresses_transition::v0::IdentityTopUpFromAddressesTransitionV0;

// =====================================
// Feature-Gated Imports
// =====================================
#[cfg(feature = "state-transition-signing")]
use {
    crate::{
        address_funds::AddressWitness,
        identity::{accessors::IdentityGettersV0, signer::Signer, Identity},
        prelude::{AddressNonce, UserFeeIncrease},
        serialization::Signable,
        state_transition::{
            address_funds_constructor_dispatch_error, consensus_errors_as_protocol_error,
            verify_address_witnesses, StateTransition, StateTransitionType,
        },
        version::FeatureVersion,
        ProtocolError,
    },
    platform_version::version::PlatformVersion,
    std::collections::BTreeMap,
};

impl IdentityTopUpFromAddressesTransitionMethodsV0 for IdentityTopUpFromAddressesTransitionV0 {
    #[cfg(feature = "state-transition-signing")]
    async fn try_from_inputs_with_signer<S: Signer<PlatformAddress>>(
        identity: &Identity,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        signer: &S,
        user_fee_increase: UserFeeIncrease,
        platform_version: &PlatformVersion,
        _version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        let mut identity_top_up_from_addresses_transition =
            IdentityTopUpFromAddressesTransitionV0 {
                inputs,
                output: None,
                identity_id: identity.id(),
                fee_strategy: vec![
                    crate::address_funds::AddressFundsFeeStrategyStep::DeductFromInput(0),
                ],
                user_fee_increase,
                input_witnesses: vec![],
            };

        if let Some(error) = address_funds_constructor_dispatch_error(
            StateTransitionType::IdentityTopUpFromAddresses,
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
        let pre_validation_result = identity_top_up_from_addresses_transition
            .validate_structure_without_input_witnesses(platform_version);
        if let Some(error) = consensus_errors_as_protocol_error(pre_validation_result) {
            return Err(error);
        }

        let state_transition: StateTransition =
            identity_top_up_from_addresses_transition.clone().into();

        let signable_bytes = state_transition.signable_bytes()?;

        let mut input_witnesses: Vec<AddressWitness> =
            Vec::with_capacity(identity_top_up_from_addresses_transition.inputs.len());
        for address in identity_top_up_from_addresses_transition.inputs.keys() {
            input_witnesses.push(signer.sign_create_witness(address, &signable_bytes).await?);
        }
        verify_address_witnesses(
            identity_top_up_from_addresses_transition.inputs.keys(),
            &input_witnesses,
            &signable_bytes,
        )?;
        identity_top_up_from_addresses_transition.input_witnesses = input_witnesses;

        // After signing, only the witness count needs (re-)validation; the rest
        // of the structure was already verified above.
        let validation_result =
            identity_top_up_from_addresses_transition.validate_input_witnesses_count();
        if let Some(error) = consensus_errors_as_protocol_error(validation_result) {
            return Err(error);
        }

        Ok(identity_top_up_from_addresses_transition.into())
    }
}

impl IdentityTopUpFromAddressesTransitionAccessorsV0 for IdentityTopUpFromAddressesTransitionV0 {
    /// Set identity id
    fn set_identity_id(&mut self, identity_id: Identifier) {
        self.identity_id = identity_id;
    }

    /// Returns identity id
    fn identity_id(&self) -> &Identifier {
        &self.identity_id
    }

    /// Get the optional output
    fn output(&self) -> Option<&(PlatformAddress, Credits)> {
        self.output.as_ref()
    }

    /// Set the optional output
    fn set_output(&mut self, output: Option<(PlatformAddress, Credits)>) {
        self.output = output;
    }
}
