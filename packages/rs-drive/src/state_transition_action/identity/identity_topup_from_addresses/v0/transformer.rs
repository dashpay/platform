use crate::state_transition_action::identity::identity_topup_from_addresses::v0::IdentityTopUpFromAddressesTransitionActionV0;
use dpp::consensus::ConsensusError;
use dpp::state_transition::state_transitions::identity::identity_topup_from_addresses_transition::v0::IdentityTopUpFromAddressesTransitionV0;

impl IdentityTopUpFromAddressesTransitionActionV0 {
    /// try from
    pub fn try_from(value: IdentityTopUpFromAddressesTransitionV0) -> Result<Self, ConsensusError> {
        let IdentityTopUpFromAddressesTransitionV0 {
            identity_id,
            inputs,
            user_fee_increase,
            ..
        } = value;

        Ok(IdentityTopUpFromAddressesTransitionActionV0 {
            inputs_with_remaining_balance,
            identity_id,
            user_fee_increase,
        })
    }

    /// try from borrowed
    pub fn try_from_borrowed(
        value: &IdentityTopUpFromAddressesTransitionV0,
    ) -> Result<Self, ConsensusError> {
        let IdentityTopUpFromAddressesTransitionV0 {
            identity_id,
            inputs,
            user_fee_increase,
            ..
        } = value;

        Ok(IdentityTopUpFromAddressesTransitionActionV0 {
            inputs_with_remaining_balance: inputs.clone(), //todo
            identity_id: *identity_id,
            user_fee_increase: *user_fee_increase,
        })
    }
}
