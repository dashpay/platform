use crate::state_transition_action::contract::contract_user_moderation::v0::ContractUserModerationTransitionActionV0;
use dpp::data_contract::config::moderation::ContractModerationStatus;
use dpp::state_transition::contract_user_moderation_transition::v0::ContractUserModerationTransitionV0;

impl ContractUserModerationTransitionActionV0 {
    /// The action of a borrowed transition, carrying the target's status as it is stored
    pub fn from_borrowed_transition_with_status(
        value: &ContractUserModerationTransitionV0,
        current_status: ContractModerationStatus,
    ) -> Self {
        let ContractUserModerationTransitionV0 {
            owner_id,
            data_contract_id,
            identity_contract_nonce,
            action,
            user_fee_increase,
            ..
        } = value;
        ContractUserModerationTransitionActionV0 {
            moderator_id: *owner_id,
            data_contract_id: *data_contract_id,
            identity_contract_nonce: *identity_contract_nonce,
            action: *action,
            current_status,
            user_fee_increase: *user_fee_increase,
        }
    }
}
