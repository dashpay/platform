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
        state_transition::StateTransition,
        version::FeatureVersion,
        ProtocolError,
    },
    platform_version::version::PlatformVersion,
    std::collections::BTreeMap,
};

impl IdentityTopUpFromAddressesTransitionMethodsV0 for IdentityTopUpFromAddressesTransitionV0 {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_inputs_with_signer<S: Signer<PlatformAddress>>(
        identity: &Identity,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        signer: &S,
        user_fee_increase: UserFeeIncrease,
        _platform_version: &PlatformVersion,
        _version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        let mut identity_top_up_from_addresses_transition =
            IdentityTopUpFromAddressesTransitionV0 {
                inputs: inputs.clone(),
                output: None,
                identity_id: identity.id(),
                fee_strategy: vec![
                    crate::address_funds::AddressFundsFeeStrategyStep::DeductFromInput(0),
                ],
                user_fee_increase,
                input_witnesses: vec![],
            };

        let state_transition: StateTransition =
            identity_top_up_from_addresses_transition.clone().into();

        let signable_bytes = state_transition.signable_bytes()?;

        identity_top_up_from_addresses_transition.input_witnesses = inputs
            .iter()
            .map(|(address, _)| signer.sign_create_witness(address, &signable_bytes))
            .collect::<Result<Vec<AddressWitness>, ProtocolError>>()?;

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
