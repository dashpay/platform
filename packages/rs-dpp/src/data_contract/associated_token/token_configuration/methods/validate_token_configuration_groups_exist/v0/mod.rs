use crate::consensus::basic::data_contract::{
    GroupPositionDoesNotExistError, MainGroupIsNotDefinedError,
};
use crate::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::group::Group;
use crate::data_contract::GroupContractPosition;
use crate::validation::SimpleConsensusValidationResult;
use std::collections::BTreeMap;

impl TokenConfiguration {
    #[inline(always)]
    pub(super) fn validate_token_config_groups_exist_v0(
        &self,
        groups: &BTreeMap<GroupContractPosition, Group>,
    ) -> SimpleConsensusValidationResult {
        // Initialize validation result
        let validation_result = SimpleConsensusValidationResult::new();

        // Collect all group positions used in the token configuration
        let (group_positions, uses_main_group) = self.all_used_group_positions();

        // Check that all referenced group positions exist in the provided groups map
        for group_position in group_positions {
            if !groups.contains_key(&group_position) {
                return SimpleConsensusValidationResult::new_with_error(
                    GroupPositionDoesNotExistError::new(group_position).into(),
                );
            }
        }

        if uses_main_group && self.main_control_group().is_none() {
            return SimpleConsensusValidationResult::new_with_error(
                MainGroupIsNotDefinedError::new().into(),
            );
        }

        // If a main group is defined in the token configuration, verify its existence
        if let Some(main_group_position) = self.main_control_group() {
            if !groups.contains_key(&main_group_position) {
                return SimpleConsensusValidationResult::new_with_error(
                    GroupPositionDoesNotExistError::new(main_group_position).into(),
                );
            }
        }

        // If we reach here with no errors, return an empty result
        validation_result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;
    use crate::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Setters;
    use crate::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
    use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
    use crate::data_contract::change_control_rules::v0::ChangeControlRulesV0;
    use crate::data_contract::change_control_rules::ChangeControlRules;
    use crate::data_contract::group::v0::GroupV0;
    use platform_value::Identifier;

    fn make_group(member_byte: u8) -> Group {
        let mut members = std::collections::BTreeMap::new();
        members.insert(Identifier::from([member_byte; 32]), 1u32);
        Group::V0(GroupV0 {
            members,
            required_power: 1,
        })
    }

    fn config_with_group_rule(position: GroupContractPosition) -> TokenConfiguration {
        let mut v0 = TokenConfigurationV0::default_most_restrictive();
        v0.set_freeze_rules(ChangeControlRules::V0(ChangeControlRulesV0 {
            authorized_to_make_change: AuthorizedActionTakers::Group(position),
            admin_action_takers: AuthorizedActionTakers::NoOne,
            changing_authorized_action_takers_to_no_one_allowed: false,
            changing_admin_action_takers_to_no_one_allowed: false,
            self_changing_admin_action_takers_allowed: false,
        }));
        TokenConfiguration::V0(v0)
    }

    #[test]
    fn empty_config_with_no_groups_validates() {
        let config = TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive());
        let groups: BTreeMap<GroupContractPosition, Group> = BTreeMap::new();
        let result = config.validate_token_config_groups_exist_v0(&groups);
        assert!(
            result.is_valid(),
            "expected valid, got errors {:?}",
            result.errors
        );
    }

    #[test]
    fn missing_referenced_group_produces_group_position_error() {
        let config = config_with_group_rule(42);
        let groups: BTreeMap<GroupContractPosition, Group> = BTreeMap::new();
        let result = config.validate_token_config_groups_exist_v0(&groups);
        assert!(!result.is_valid());
        assert_eq!(result.errors.len(), 1);
        match result.errors.first() {
            Some(ConsensusError::BasicError(BasicError::GroupPositionDoesNotExistError(_))) => {}
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[test]
    fn referenced_group_present_validates() {
        let config = config_with_group_rule(7);
        let mut groups: BTreeMap<GroupContractPosition, Group> = BTreeMap::new();
        groups.insert(7, make_group(1));
        let result = config.validate_token_config_groups_exist_v0(&groups);
        assert!(
            result.is_valid(),
            "expected valid, got errors {:?}",
            result.errors
        );
    }

    #[test]
    fn main_group_used_but_main_control_group_not_set_produces_main_group_not_defined() {
        let mut v0 = TokenConfigurationV0::default_most_restrictive();
        v0.set_freeze_rules(ChangeControlRules::V0(ChangeControlRulesV0 {
            authorized_to_make_change: AuthorizedActionTakers::MainGroup,
            admin_action_takers: AuthorizedActionTakers::NoOne,
            changing_authorized_action_takers_to_no_one_allowed: false,
            changing_admin_action_takers_to_no_one_allowed: false,
            self_changing_admin_action_takers_allowed: false,
        }));
        // Leave main_control_group = None
        let config = TokenConfiguration::V0(v0);
        let groups: BTreeMap<GroupContractPosition, Group> = BTreeMap::new();
        let result = config.validate_token_config_groups_exist_v0(&groups);
        assert!(!result.is_valid());
        match result.errors.first() {
            Some(ConsensusError::BasicError(BasicError::MainGroupIsNotDefinedError(_))) => {}
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[test]
    fn main_control_group_set_but_not_in_groups_map_errors() {
        let mut v0 = TokenConfigurationV0::default_most_restrictive();
        v0.set_main_control_group(Some(99));
        let config = TokenConfiguration::V0(v0);
        let groups: BTreeMap<GroupContractPosition, Group> = BTreeMap::new();
        let result = config.validate_token_config_groups_exist_v0(&groups);
        assert!(!result.is_valid());
        match result.errors.first() {
            Some(ConsensusError::BasicError(BasicError::GroupPositionDoesNotExistError(_))) => {}
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[test]
    fn main_control_group_set_and_in_groups_map_validates() {
        let mut v0 = TokenConfigurationV0::default_most_restrictive();
        v0.set_main_control_group(Some(3));
        let config = TokenConfiguration::V0(v0);
        let mut groups: BTreeMap<GroupContractPosition, Group> = BTreeMap::new();
        groups.insert(3, make_group(2));
        let result = config.validate_token_config_groups_exist_v0(&groups);
        assert!(
            result.is_valid(),
            "expected valid, got errors {:?}",
            result.errors
        );
    }

    #[test]
    fn main_group_referenced_and_main_control_group_set_and_present_validates() {
        let mut v0 = TokenConfigurationV0::default_most_restrictive();
        v0.set_main_control_group(Some(5));
        v0.set_freeze_rules(ChangeControlRules::V0(ChangeControlRulesV0 {
            authorized_to_make_change: AuthorizedActionTakers::MainGroup,
            admin_action_takers: AuthorizedActionTakers::NoOne,
            changing_authorized_action_takers_to_no_one_allowed: false,
            changing_admin_action_takers_to_no_one_allowed: false,
            self_changing_admin_action_takers_allowed: false,
        }));
        let config = TokenConfiguration::V0(v0);
        let mut groups: BTreeMap<GroupContractPosition, Group> = BTreeMap::new();
        groups.insert(5, make_group(4));
        let result = config.validate_token_config_groups_exist_v0(&groups);
        assert!(
            result.is_valid(),
            "expected valid, got errors {:?}",
            result.errors
        );
    }
}
