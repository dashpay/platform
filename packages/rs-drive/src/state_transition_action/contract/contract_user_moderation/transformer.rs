use crate::state_transition_action::contract::contract_user_moderation::v0::ContractUserModerationTransitionActionV0;
use crate::state_transition_action::contract::contract_user_moderation::ContractUserModerationTransitionAction;
use dpp::data_contract::config::moderation::ContractModerationStatus;
use dpp::state_transition::contract_user_moderation_transition::ContractUserModerationTransition;

impl ContractUserModerationTransitionAction {
    /// The action of a borrowed transition, carrying the target's status as it is stored
    pub fn from_borrowed_transition_with_status(
        value: &ContractUserModerationTransition,
        current_status: ContractModerationStatus,
    ) -> Self {
        match value {
            ContractUserModerationTransition::V0(v0) => {
                ContractUserModerationTransitionActionV0::from_borrowed_transition_with_status(
                    v0,
                    current_status,
                )
                .into()
            }
        }
    }
}
