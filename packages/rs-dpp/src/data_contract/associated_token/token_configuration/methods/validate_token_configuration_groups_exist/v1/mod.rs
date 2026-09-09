use crate::consensus::basic::data_contract::{
    GroupPositionDoesNotExistError, MainGroupIsNotDefinedError,
};
use crate::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
use crate::data_contract::group::Group;
use crate::data_contract::GroupContractPosition;
use crate::validation::SimpleConsensusValidationResult;
use std::collections::BTreeMap;

impl TokenConfiguration {
    #[inline(always)]
    pub(super) fn validate_token_config_groups_exist_v1(
        &self,
        groups: &BTreeMap<GroupContractPosition, Group>,
    ) -> SimpleConsensusValidationResult {
        let legacy_result = self.validate_token_config_groups_exist_v0(groups);
        if !legacy_result.is_valid() {
            return legacy_result;
        }

        let TokenConfiguration::V0(configuration) = self;

        for (_, rules) in configuration.all_change_control_rules() {
            for action_takers in [
                rules.authorized_to_make_change_action_takers(),
                rules.admin_action_takers(),
            ] {
                match action_takers {
                    AuthorizedActionTakers::Group(group_position)
                        if !groups.contains_key(group_position) =>
                    {
                        return SimpleConsensusValidationResult::new_with_error(
                            GroupPositionDoesNotExistError::new(*group_position).into(),
                        );
                    }
                    AuthorizedActionTakers::MainGroup
                        if configuration.main_control_group().is_none() =>
                    {
                        return SimpleConsensusValidationResult::new_with_error(
                            MainGroupIsNotDefinedError::new().into(),
                        );
                    }
                    _ => {}
                }
            }
        }

        SimpleConsensusValidationResult::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;
    use crate::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
    use crate::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
    use crate::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Setters;
    use crate::data_contract::change_control_rules::v0::ChangeControlRulesV0;
    use crate::data_contract::change_control_rules::ChangeControlRules;

    fn pricing_rules(action_takers: AuthorizedActionTakers) -> ChangeControlRules {
        ChangeControlRules::V0(ChangeControlRulesV0 {
            authorized_to_make_change: action_takers,
            admin_action_takers: AuthorizedActionTakers::NoOne,
            changing_authorized_action_takers_to_no_one_allowed: false,
            changing_admin_action_takers_to_no_one_allowed: false,
            self_changing_admin_action_takers_allowed: false,
        })
    }

    #[test]
    fn direct_purchase_pricing_main_group_is_validated() {
        let mut configuration = TokenConfigurationV0::default_most_restrictive();
        configuration
            .distribution_rules_mut()
            .set_change_direct_purchase_pricing_rules(pricing_rules(
                AuthorizedActionTakers::MainGroup,
            ));
        let configuration = TokenConfiguration::V0(configuration);
        let groups = BTreeMap::new();

        assert!(configuration
            .validate_token_config_groups_exist_v0(&groups)
            .is_valid());

        let result = configuration.validate_token_config_groups_exist_v1(&groups);
        assert!(matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::MainGroupIsNotDefinedError(_)
            )]
        ));
    }

    #[test]
    fn direct_purchase_pricing_explicit_group_must_exist() {
        let mut configuration = TokenConfigurationV0::default_most_restrictive();
        configuration
            .distribution_rules_mut()
            .set_change_direct_purchase_pricing_rules(pricing_rules(
                AuthorizedActionTakers::Group(7),
            ));
        let configuration = TokenConfiguration::V0(configuration);
        let groups = BTreeMap::new();

        let result = configuration.validate_token_config_groups_exist_v1(&groups);
        assert!(matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(BasicError::GroupPositionDoesNotExistError(error))]
                if error.missing_group_position() == 7
        ));
    }
}
