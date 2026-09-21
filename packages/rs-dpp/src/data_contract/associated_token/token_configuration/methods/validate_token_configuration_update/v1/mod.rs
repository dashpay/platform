use crate::consensus::basic::data_contract::DataContractTokenConfigurationUpdateError;
use crate::data_contract::associated_token::token_configuration::accessors::v1::TokenConfigurationV1Getters;
use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::group::Group;
use crate::data_contract::GroupContractPosition;
use crate::group::action_taker::{ActionGoal, ActionTaker};
use crate::validation::SimpleConsensusValidationResult;
use platform_value::Identifier;
use std::collections::BTreeMap;

impl TokenConfiguration {
    pub(super) fn validate_token_config_update_v1(
        &self,
        new_config: &TokenConfiguration,
        contract_owner_id: &Identifier,
        groups: &BTreeMap<GroupContractPosition, Group>,
        action_taker: &ActionTaker,
        goal: ActionGoal,
    ) -> SimpleConsensusValidationResult {
        let result = self.validate_token_config_update_v0(
            new_config,
            contract_owner_id,
            groups,
            action_taker,
            goal,
        );
        if !result.is_valid() {
            return result;
        }
        // The shielded pool opt-in is decided at creation: enabling it later would need the
        // pool subtree created by the update, and a pool holding notes can never be removed.
        if self.has_shielded_pool() != new_config.has_shielded_pool() {
            return SimpleConsensusValidationResult::new_with_error(
                DataContractTokenConfigurationUpdateError::new(
                    "update".to_string(),
                    "hasShieldedPool".to_string(),
                    self.clone(),
                    new_config.clone(),
                )
                .into(),
            );
        }

        SimpleConsensusValidationResult::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::associated_token::token_configuration::accessors::v1::TokenConfigurationV1Setters;
    use crate::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
    use platform_version::version::PlatformVersion;

    #[test]
    fn should_freeze_the_shielded_pool_opt_in_only_from_protocol_15() {
        let owner = Identifier::from([1; 32]);
        let groups = BTreeMap::new();
        let action_taker = ActionTaker::SingleIdentity(owner);
        let plain = TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive());
        let mut shielded = plain.clone();
        shielded.set_has_shielded_pool(true);

        for platform_version in [PlatformVersion::get(14).unwrap(), PlatformVersion::latest()] {
            for (old, new) in [
                (&plain, &shielded),
                (&shielded, &plain),
                (&shielded, &shielded),
            ] {
                let result = old
                    .validate_token_config_update(
                        new,
                        &owner,
                        &groups,
                        &action_taker,
                        ActionGoal::ActionCompletion,
                        platform_version,
                    )
                    .expect("validate token configuration update");
                assert_eq!(
                    result.is_valid(),
                    platform_version.protocol_version < 15
                        || old.has_shielded_pool() == new.has_shielded_pool(),
                    "unexpected update validation on protocol {}: {:?}",
                    platform_version.protocol_version,
                    result.errors,
                );
            }
        }
    }
}
