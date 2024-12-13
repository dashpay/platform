pub mod authorized_action_takers;
mod v0;

use crate::data_contract::change_control_rules::v0::ChangeControlRulesV0;
use crate::data_contract::group::Group;
use crate::multi_identity_events::ActionTaker;
use bincode::{Decode, Encode};
use platform_value::Identifier;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq)]
pub enum ChangeControlRules {
    V0(ChangeControlRulesV0),
}

impl ChangeControlRules {
    pub fn can_change_to(
        &self,
        other: &ChangeControlRules,
        contract_owner_id: &Identifier,
        main_group: &Group,
        action_taker: &ActionTaker,
    ) -> bool {
        match (self, other) {
            (ChangeControlRules::V0(v0), ChangeControlRules::V0(v0_other)) => {
                v0.can_change_to(v0_other, contract_owner_id, main_group, action_taker)
            }
        }
    }
}
