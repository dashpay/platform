use crate::state_transition_action::identity::identity_create_from_addresses::v0::IdentityCreateFromAddressesTransitionActionV0;
use dpp::consensus::basic::value_error::ValueError;
use dpp::consensus::ConsensusError;
use dpp::state_transition::state_transitions::identity::identity_create_from_addresses_transition::v0::IdentityCreateFromAddressesTransitionV0;
use dpp::state_transition::StateTransitionIdentityIdFromInputs;
use dpp::state_transition::StateTransitionWitnessSigned;

impl IdentityCreateFromAddressesTransitionActionV0 {
    /// try from
    pub fn try_from(
        value: IdentityCreateFromAddressesTransitionV0,
    ) -> Result<Self, ConsensusError> {
        let identity_id = value.identity_id_from_inputs().map_err(|e| {
            ConsensusError::from(ValueError::new_from_string(format!(
                "Failed to calculate identity id from inputs: {}",
                e
            )))
        })?;

        // Calculate fund_identity_amount from all inputs
        let fund_identity_amount = value.inputs().values().map(|(_, credits)| credits).sum();

        let IdentityCreateFromAddressesTransitionV0 {
            inputs,
            output,
            public_keys,
            user_fee_increase,
            ..
        } = value;

        Ok(IdentityCreateFromAddressesTransitionActionV0 {
            inputs_with_remaining_balance: inputs,
            output,
            public_keys: public_keys.into_iter().map(|a| a.into()).collect(),
            identity_id,
            fund_identity_amount,
            user_fee_increase,
        })
    }

    /// try from borrowed
    pub fn try_from_borrowed(
        value: &IdentityCreateFromAddressesTransitionV0,
    ) -> Result<Self, ConsensusError> {
        let identity_id = value.identity_id_from_inputs().map_err(|e| {
            ConsensusError::from(ValueError::new_from_string(format!(
                "Failed to calculate identity id from inputs: {}",
                e
            )))
        })?;

        // Calculate fund_identity_amount from all inputs
        let fund_identity_amount = value.inputs().values().map(|(_, credits)| credits).sum();

        let IdentityCreateFromAddressesTransitionV0 {
            inputs,
            output,
            public_keys,
            user_fee_increase,
            ..
        } = value;

        Ok(IdentityCreateFromAddressesTransitionActionV0 {
            inputs_with_remaining_balance: inputs.clone(),
            output: output.clone(),
            public_keys: public_keys.iter().map(|key| key.into()).collect(),
            identity_id,
            fund_identity_amount,
            user_fee_increase: *user_fee_increase,
        })
    }
}
