pub mod authorized_action_takers;
pub mod v0;

use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
use crate::data_contract::change_control_rules::v0::ChangeControlRulesV0;
use crate::data_contract::group::Group;
use crate::data_contract::GroupContractPosition;
use crate::multi_identity_events::ActionTaker;
use bincode::{Decode, Encode};
use derive_more::From;
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq, From)]
pub enum ChangeControlRules {
    V0(ChangeControlRulesV0),
}

impl ChangeControlRules {
    pub fn authorized_to_make_change_action_takers(&self) -> &AuthorizedActionTakers {
        match self {
            ChangeControlRules::V0(v0) => &v0.authorized_to_make_change,
        }
    }
    pub fn can_make_change(
        &self,
        contract_owner_id: &Identifier,
        main_group: Option<&Group>,
        groups: &BTreeMap<GroupContractPosition, Group>,
        action_taker: &ActionTaker,
    ) -> bool {
        match self {
            ChangeControlRules::V0(v0) => {
                v0.can_make_change(contract_owner_id, main_group, groups, action_taker)
            }
        }
    }
    pub fn can_change_to(
        &self,
        other: &ChangeControlRules,
        contract_owner_id: &Identifier,
        main_group: Option<&Group>,
        groups: &BTreeMap<GroupContractPosition, Group>,
        action_taker: &ActionTaker,
    ) -> bool {
        match (self, other) {
            (ChangeControlRules::V0(v0), ChangeControlRules::V0(v0_other)) => v0.can_change_to(
                v0_other,
                contract_owner_id,
                main_group,
                groups,
                action_taker,
            ),
        }
    }
}
