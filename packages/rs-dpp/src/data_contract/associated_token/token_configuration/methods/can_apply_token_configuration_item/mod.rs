use crate::data_contract::associated_token::token_configuration::v1::TokenConfigurationV1;
use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::associated_token::token_configuration_item::{
    token_configuration_v0_change_items, TokenConfigurationChangeItem,
};
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
            TokenConfiguration::V1(v1) => v1.can_apply_token_configuration_item(
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

impl TokenConfigurationV1 {
    /// Determines whether a `TokenConfigurationChangeItem` can be applied: the shielded pool's
    /// threshold items under `minimum_pool_notes_for_outgoing_change_rules`, every other item
    /// under the nested V0 configuration's rules.
    pub fn can_apply_token_configuration_item(
        &self,
        change_item: &TokenConfigurationChangeItem,
        contract_owner_id: &Identifier,
        main_group: Option<GroupContractPosition>,
        groups: &BTreeMap<GroupContractPosition, Group>,
        action_taker: &ActionTaker,
        goal: ActionGoal,
    ) -> bool {
        let rules = &self.minimum_pool_notes_for_outgoing_change_rules;
        match change_item {
            TokenConfigurationChangeItem::MinimumPoolNotesForOutgoing(_) => {
                rules.can_make_change(contract_owner_id, main_group, groups, action_taker, goal)
            }
            TokenConfigurationChangeItem::MinimumPoolNotesForOutgoingControlGroup(
                control_group,
            ) => rules.can_change_authorized_action_takers(
                control_group,
                contract_owner_id,
                main_group,
                groups,
                action_taker,
                goal,
            ),
            TokenConfigurationChangeItem::MinimumPoolNotesForOutgoingAdminGroup(admin_group) => {
                rules.can_change_admin_action_takers(
                    admin_group,
                    contract_owner_id,
                    main_group,
                    groups,
                    action_taker,
                    goal,
                )
            }
            token_configuration_v0_change_items!() => self.base.can_apply_token_configuration_item(
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
