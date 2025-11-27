use crate::state_transition_action::identity::identity_create_from_addresses::v0::IdentityCreateFromAddressesTransitionActionV0;
use dpp::consensus::ConsensusError;
use dpp::state_transition::state_transitions::identity::identity_create_from_addresses_transition::v0::IdentityCreateFromAddressesTransitionV0;

impl IdentityCreateFromAddressesTransitionActionV0 {
    /// try from
    pub fn try_from(
        value: IdentityCreateFromAddressesTransitionV0,
    ) -> Result<Self, ConsensusError> {
        let IdentityCreateFromAddressesTransitionV0 {
            inputs,
            public_keys,
            user_fee_increase,
            ..
        } = value;

        Ok(IdentityCreateFromAddressesTransitionActionV0 {
            inputs_with_remaining_balance,
            public_keys: public_keys.into_iter().map(|a| a.into()).collect(),
            identity_id,
            user_fee_increase,
        })
    }

    /// try from borrowed
    pub fn try_from_borrowed(
        value: &IdentityCreateFromAddressesTransitionV0,
    ) -> Result<Self, ConsensusError> {
        let IdentityCreateFromAddressesTransitionV0 {
            inputs,
            public_keys,
            user_fee_increase,
            ..
        } = value;

        Ok(IdentityCreateFromAddressesTransitionActionV0 {
            inputs_with_remaining_balance: inputs.clone(),
            public_keys: public_keys.iter().map(|key| key.into()).collect(),
            identity_id: *identity_id,
            user_fee_increase: *user_fee_increase,
        })
    }
}
