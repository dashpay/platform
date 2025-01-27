pub mod authorized_action_takers;
pub mod v0;

use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
use crate::data_contract::change_control_rules::v0::ChangeControlRulesV0;
use crate::data_contract::group::Group;
use crate::data_contract::GroupContractPosition;
use crate::group::action_taker::{ActionGoal, ActionTaker};
use bincode::{Decode, Encode};
use derive_more::From;
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq, From)]
pub enum ChangeControlRules {
    V0(ChangeControlRulesV0),
}

impl ChangeControlRules {
    pub fn admin_action_takers(&self) -> &AuthorizedActionTakers {
        match self {
            ChangeControlRules::V0(v0) => &v0.admin_action_takers,
        }
    }
    pub fn authorized_to_make_change_action_takers(&self) -> &AuthorizedActionTakers {
        match self {
            ChangeControlRules::V0(v0) => &v0.authorized_to_make_change,
        }
    }

    pub fn set_admin_action_takers(&mut self, admin_action_takers: AuthorizedActionTakers) {
        match self {
            ChangeControlRules::V0(v0) => {
                v0.admin_action_takers = admin_action_takers;
            }
        }
    }

    pub fn set_authorized_to_make_change_action_takers(
        &mut self,
        authorized_to_make_change: AuthorizedActionTakers,
    ) {
        match self {
            ChangeControlRules::V0(v0) => {
                v0.authorized_to_make_change = authorized_to_make_change;
            }
        }
    }

    pub fn can_make_change(
        &self,
        contract_owner_id: &Identifier,
        main_group: Option<GroupContractPosition>,
        groups: &BTreeMap<GroupContractPosition, Group>,
        action_taker: &ActionTaker,
        goal: ActionGoal,
    ) -> bool {
        match self {
            ChangeControlRules::V0(v0) => {
                v0.can_make_change(contract_owner_id, main_group, groups, action_taker, goal)
            }
        }
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
        match self {
            ChangeControlRules::V0(v0) => v0.can_change_authorized_action_takers(
                controlling_action_takers,
                contract_owner_id,
                main_group,
                groups,
                action_taker,
                goal,
            ),
        }
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
        match self {
            ChangeControlRules::V0(v0) => v0.can_change_admin_action_takers(
                admin_action_takers,
                contract_owner_id,
                main_group,
                groups,
                action_taker,
                goal,
            ),
        }
    }
    pub fn can_change_to(
        &self,
        other: &ChangeControlRules,
        contract_owner_id: &Identifier,
        main_group: Option<GroupContractPosition>,
        groups: &BTreeMap<GroupContractPosition, Group>,
        action_taker: &ActionTaker,
        goal: ActionGoal,
    ) -> bool {
        match (self, other) {
            (ChangeControlRules::V0(v0), ChangeControlRules::V0(v0_other)) => v0.can_change_to(
                v0_other,
                contract_owner_id,
                main_group,
                groups,
                action_taker,
                goal,
            ),
        }
    }
}

impl fmt::Display for ChangeControlRules {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChangeControlRules::V0(v0) => {
                write!(f, "{}", v0) //just pass through
            }
        }
    }
}
