use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use crate::data_contract::group::Group;
use crate::data_contract::GroupContractPosition;
use crate::group::action_taker::{ActionGoal, ActionTaker};
use platform_value::Identifier;
use std::collections::BTreeMap;

mod v0;

impl TokenConfiguration {
    /// Applies a `TokenConfigurationChangeItem` to this token configuration.
    ///
    /// # Parameters
    /// - `change_item`: The change item to be applied.
    ///
    /// This method modifies the current `TokenConfigurationV0` instance in place.
    pub fn can_apply_token_configuration_item(
        &self,
        change_item: &TokenConfigurationChangeItem,
        contract_owner_id: &Identifier,
        main_group: Option<GroupContractPosition>,
        groups: &BTreeMap<GroupContractPosition, Group>,
        action_taker: &ActionTaker,
        goal: ActionGoal,
    ) -> bool {
        match self {
            TokenConfiguration::V0(v0) => v0.can_apply_token_configuration_item(
                change_item,
                contract_owner_id,
                main_group,
                groups,
                action_taker,
                goal,
            ),
        }
    }
}
