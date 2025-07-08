use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
use crate::data_contract::group::Group;
use crate::data_contract::GroupContractPosition;
use crate::group::action_taker::{ActionGoal, ActionTaker};
use bincode::{Decode, Encode};
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq, Default)]
pub struct ChangeControlRulesV0 {
    /// This is who is authorized to make such a change
    pub authorized_to_make_change: AuthorizedActionTakers,
    /// This is who is authorized to make such a change to the people authorized to make a change
    pub admin_action_takers: AuthorizedActionTakers,
    /// Are we allowed to change to None in the future
    pub changing_authorized_action_takers_to_no_one_allowed: bool,
    /// Are we allowed to change the admin action takers to no one in the future
    pub changing_admin_action_takers_to_no_one_allowed: bool,
    /// Can the admin action takers change themselves
    pub self_changing_admin_action_takers_allowed: bool,
}

impl ChangeControlRulesV0 {
    pub fn can_make_change(
        &self,
        contract_owner_id: &Identifier,
        main_group: Option<GroupContractPosition>,
        groups: &BTreeMap<GroupContractPosition, Group>,
        action_taker: &ActionTaker,
        goal: ActionGoal,
    ) -> bool {
        self.authorized_to_make_change.allowed_for_action_taker(
            contract_owner_id,
            main_group,
            groups,
            action_taker,
            goal,
        )
    }
    pub fn can_change_authorized_action_takers(
        &self,
        controlling_action_takers: &AuthorizedActionTakers,
        contract_owner_id: &Identifier,
        main_group: Option<GroupContractPosition>,
        groups: &BTreeMap<GroupContractPosition, Group>,
        action_taker: &ActionTaker,
        goal: ActionGoal,
    ) -> bool {
        if !self.changing_authorized_action_takers_to_no_one_allowed
            && controlling_action_takers == &AuthorizedActionTakers::NoOne
        {
            return false;
        }
        self.admin_action_takers.allowed_for_action_taker(
            contract_owner_id,
            main_group,
            groups,
            action_taker,
            goal,
        )
    }

    pub fn can_change_admin_action_takers(
        &self,
        admin_action_takers: &AuthorizedActionTakers,
        contract_owner_id: &Identifier,
        main_group: Option<GroupContractPosition>,
        groups: &BTreeMap<GroupContractPosition, Group>,
        action_taker: &ActionTaker,
        goal: ActionGoal,
    ) -> bool {
        if !self.self_changing_admin_action_takers_allowed {
            return false;
        }
        if !self.changing_admin_action_takers_to_no_one_allowed
            && admin_action_takers == &AuthorizedActionTakers::NoOne
        {
            return false;
        }
        self.admin_action_takers.allowed_for_action_taker(
            contract_owner_id,
            main_group,
            groups,
            action_taker,
            goal,
        )
    }

    pub fn can_change_to(
        &self,
        other: &ChangeControlRulesV0,
        contract_owner_id: &Identifier,
        main_group: Option<GroupContractPosition>,
        groups: &BTreeMap<GroupContractPosition, Group>,
        action_taker: &ActionTaker,
        goal: ActionGoal,
    ) -> bool {
        // First, check if the action taker is allowed to make any changes at all
        if !self.authorized_to_make_change.allowed_for_action_taker(
            contract_owner_id,
            main_group,
            groups,
            action_taker,
            goal,
        ) {
            return false;
        }

        // Check if authorized_to_make_change is being modified
        if self.authorized_to_make_change != other.authorized_to_make_change {
            // Changing the authorized action takers requires the action_taker to be allowed by
            // authorized_to_change_authorized_action_takers in the current rules
            if !self.admin_action_takers.allowed_for_action_taker(
                contract_owner_id,
                main_group,
                groups,
                action_taker,
                goal,
            ) {
                return false;
            }

            // If we are changing to NoOne, ensure it's allowed
            if let AuthorizedActionTakers::NoOne = other.authorized_to_make_change {
                if !self.changing_authorized_action_takers_to_no_one_allowed {
                    return false;
                }
            }
        }

        // Check if authorized_to_change_authorized_action_takers is being modified
        if self.admin_action_takers != other.admin_action_takers {
            if !self.self_changing_admin_action_takers_allowed {
                return false;
            }

            // Must be allowed by the current authorized_to_change_authorized_action_takers
            if !self.admin_action_takers.allowed_for_action_taker(
                contract_owner_id,
                main_group,
                groups,
                action_taker,
                goal,
            ) {
                return false;
            }

            // If we are changing to NoOne, ensure it's allowed
            if let AuthorizedActionTakers::NoOne = other.admin_action_takers {
                if !self.changing_admin_action_takers_to_no_one_allowed {
                    return false;
                }
            }
        }

        // If we reach here, the changes are allowed
        true
    }
}

impl fmt::Display for ChangeControlRulesV0 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ChangeControlRulesV0 {{\n  \
            authorized_to_make_change: {},\n  \
            admin_action_takers: {},\n  \
            changing_authorized_action_takers_to_no_one_allowed: {},\n  \
            changing_admin_action_takers_to_no_one_allowed: {},\n  \
            self_changing_admin_action_takers_allowed: {}\n\
            }}",
            self.authorized_to_make_change,
            self.admin_action_takers,
            self.changing_authorized_action_takers_to_no_one_allowed,
            self.changing_admin_action_takers_to_no_one_allowed,
            self.self_changing_admin_action_takers_allowed
        )
    }
}
