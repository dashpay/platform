use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
use crate::data_contract::group::Group;
use crate::multi_identity_events::ActionTaker;
use bincode::{Decode, Encode};
use platform_value::Identifier;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq)]
pub struct ChangeControlRulesV0 {
    /// This is who is authorized to make such a change
    authorized_to_make_change: AuthorizedActionTakers,
    /// This is who is authorized to make such a change to the people authorized to make a change
    authorized_to_change_authorized_action_takers: AuthorizedActionTakers,
    /// Are we allowed to change to None in the future
    changing_authorized_action_takers_to_no_one_allowed: bool,
    /// Are we allowed to change to None in the future
    changing_authorized_action_takers_to_contract_owner_allowed: bool,
}

impl ChangeControlRulesV0 {
    pub fn can_change_to(
        &self,
        other: &ChangeControlRulesV0,
        contract_owner_id: &Identifier,
        main_group: &Group,
        action_taker: &ActionTaker,
    ) -> bool {
        // First, check if the action taker is allowed to make any changes at all
        if !self.authorized_to_make_change.allowed_for_action_taker(
            contract_owner_id,
            main_group,
            action_taker,
        ) {
            return false;
        }

        // Check if authorized_to_make_change is being modified
        if self.authorized_to_make_change != other.authorized_to_make_change {
            // Changing the authorized action takers requires the action_taker to be allowed by
            // authorized_to_change_authorized_action_takers in the current rules
            if !self
                .authorized_to_change_authorized_action_takers
                .allowed_for_action_taker(contract_owner_id, main_group, action_taker)
            {
                return false;
            }

            // If we are changing to NoOne, ensure it's allowed
            if let AuthorizedActionTakers::NoOne = other.authorized_to_make_change {
                if !self.changing_authorized_action_takers_to_no_one_allowed {
                    return false;
                }
            }

            // If we are changing to ContractOwner, ensure it's allowed
            if let AuthorizedActionTakers::ContractOwner = other.authorized_to_make_change {
                if !self.changing_authorized_action_takers_to_contract_owner_allowed {
                    return false;
                }
            }
        }

        // Check if authorized_to_change_authorized_action_takers is being modified
        if self.authorized_to_change_authorized_action_takers
            != other.authorized_to_change_authorized_action_takers
        {
            // Must be allowed by the current authorized_to_change_authorized_action_takers
            if !self
                .authorized_to_change_authorized_action_takers
                .allowed_for_action_taker(contract_owner_id, main_group, action_taker)
            {
                return false;
            }

            // If we are changing to NoOne, ensure it's allowed
            if let AuthorizedActionTakers::NoOne =
                other.authorized_to_change_authorized_action_takers
            {
                if !self.changing_authorized_action_takers_to_no_one_allowed {
                    return false;
                }
            }

            // If we are changing to ContractOwner, ensure it's allowed
            if let AuthorizedActionTakers::ContractOwner =
                other.authorized_to_change_authorized_action_takers
            {
                if !self.changing_authorized_action_takers_to_contract_owner_allowed {
                    return false;
                }
            }
        }

        // If we reach here, the changes are allowed
        true
    }
}
