use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use dpp::consensus::basic::data_contract::{
    InvalidDataContractIdError, InvalidDataContractVersionError, InvalidTokenBaseSupplyError,
    NonContiguousContractTokenPositionsError,
};
use dpp::consensus::basic::BasicError;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::{TokenContractPosition, INITIAL_DATA_CONTRACT_VERSION};
use dpp::prelude::DataContract;
use dpp::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::validation::ConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;
use drive::state_transition_action::StateTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::data_contract_update) trait DataContractUpdatedStateTransitionAdvancedStructureValidationV0 {
    fn validate_advanced_structure_v0(&self, execution_context: &mut StateTransitionExecutionContext, platform_version: &PlatformVersion) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl DataContractUpdatedStateTransitionAdvancedStructureValidationV0
    for DataContractUpdateTransition
{
    fn validate_advanced_structure_v0(
        &self,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        if self.data_contract().version() == INITIAL_DATA_CONTRACT_VERSION {
            let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(
                    self,
                ),
            );

            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action,
                vec![InvalidDataContractVersionError::new(2, INITIAL_DATA_CONTRACT_VERSION).into()],
            ));
        }

        // Validate data contract id
        let generated_id = DataContract::generate_data_contract_id_v0(
            self.data_contract().owner_id(),
            self.identity_contract_nonce(),
        );

        // This hash will only take 1 block (64 bytes)
        execution_context.add_operation(ValidationOperation::DoubleSha256(1));

        if generated_id != self.data_contract().id() {
            let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(
                    self,
                ),
            );

            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action,
                vec![
                    BasicError::InvalidDataContractIdError(InvalidDataContractIdError::new(
                        generated_id.to_vec(),
                        self.data_contract().id().to_vec(),
                    ))
                    .into(),
                ],
            ));
        }

        let groups = self.data_contract().groups();
        if !groups.is_empty() {
            let validation_result = DataContract::validate_groups(groups, platform_version)?;

            if !validation_result.is_valid() {
                let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                    BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(self),
                );

                return Ok(ConsensusValidationResult::new_with_data_and_errors(
                    bump_action,
                    validation_result.errors,
                ));
            }
        }

        for (expected_position, (token_contract_position, token_configuration)) in
            self.data_contract().tokens().iter().enumerate()
        {
            if expected_position as TokenContractPosition != *token_contract_position {
                let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                    BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(self),
                );

                return Ok(ConsensusValidationResult::new_with_data_and_errors(
                    bump_action,
                    vec![NonContiguousContractTokenPositionsError::new(
                        expected_position as TokenContractPosition,
                        *token_contract_position,
                    )
                    .into()],
                ));
            }

            if token_configuration.base_supply() > i64::MAX as u64 {
                let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                    BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(self),
                );

                return Ok(ConsensusValidationResult::new_with_data_and_errors(
                    bump_action,
                    vec![
                        InvalidTokenBaseSupplyError::new(token_configuration.base_supply()).into(),
                    ],
                ));
            }

            let validation_result = token_configuration
                .conventions()
                .validate_localizations(platform_version)?;
            if !validation_result.is_valid() {
                let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                    BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(self),
                );

                return Ok(ConsensusValidationResult::new_with_data_and_errors(
                    bump_action,
                    validation_result.errors,
                ));
            }

            let validation_result = token_configuration.validate_token_config_groups_exist(
                self.data_contract().groups(),
                platform_version,
            )?;
            if !validation_result.is_valid() {
                let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                    BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(self),
                );

                return Ok(ConsensusValidationResult::new_with_data_and_errors(
                    bump_action,
                    validation_result.errors,
                ));
            }
        }

        Ok(ConsensusValidationResult::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;

    mod validate_advanced_structure_for_contract_update {
        use std::collections::BTreeMap;

        use super::*;
        use dpp::consensus::ConsensusError;
        use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
        use dpp::data_contract::accessors::v1::DataContractV1Setters;
        use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Setters;
        use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
        use dpp::data_contract::associated_token::token_configuration_convention::accessors::v0::TokenConfigurationConventionV0Getters;
        use dpp::data_contract::associated_token::token_configuration_localization::v0::TokenConfigurationLocalizationV0;
        use dpp::data_contract::associated_token::token_configuration_localization::TokenConfigurationLocalization;
        use dpp::data_contract::TokenConfiguration;
        use dpp::prelude::{Identifier, IdentityNonce};
        use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransitionV0;
        use dpp::tests::fixtures::get_data_contract_fixture;
        use drive::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceActionAccessorsV0;
        use platform_version::version::PlatformVersion;
        use platform_version::{DefaultForPlatformVersion, TryIntoPlatformVersioned};

        fn make_update_transition(
            data_contract: DataContract,
            identity_contract_nonce: IdentityNonce,
            platform_version: &PlatformVersion,
        ) -> DataContractUpdateTransition {
            let mut data_contract = data_contract
                .as_v1()
                .expect("expected to convert to v1")
                .clone();

            let id = DataContract::generate_data_contract_id_v0(
                data_contract.owner_id(),
                identity_contract_nonce,
            );
            data_contract.set_id(id);
            data_contract.set_version(2);

            let versioned = data_contract
                .try_into_platform_versioned(platform_version)
                .expect("should convert");

            DataContractUpdateTransitionV0 {
                data_contract: versioned,
                identity_contract_nonce,
                user_fee_increase: 0,
                signature: Default::default(),
                signature_public_key_id: 0,
            }
            .into()
        }

        #[test]
        fn should_fail_if_base_supply_exceeds_max() {
            let platform_version = PlatformVersion::latest();
            let identity_nonce = 321;

            let mut contract =
                get_data_contract_fixture(None, identity_nonce, platform_version.protocol_version)
                    .data_contract_owned();

            let mut token =
                TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive());
            token.set_base_supply(i64::MAX as u64 + 1);
            contract.set_tokens(BTreeMap::from([(0, token)]));

            let transition = make_update_transition(contract, identity_nonce, platform_version);
            let mut context =
                StateTransitionExecutionContext::default_for_platform_version(platform_version)
                    .unwrap();

            let result = transition
                .validate_advanced_structure_v0(&mut context, platform_version)
                .unwrap();

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::InvalidTokenBaseSupplyError(_)
                )]
            );
        }

        #[test]
        fn should_fail_if_localization_invalid() {
            let platform_version = PlatformVersion::latest();
            let identity_nonce = 654;

            let mut contract =
                get_data_contract_fixture(None, identity_nonce, platform_version.protocol_version)
                    .data_contract_owned();

            let mut token =
                TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive());
            let localizations = token.conventions_mut().localizations_mut();
            localizations.clear();
            localizations.insert(
                "".to_string(),
                TokenConfigurationLocalization::V0(TokenConfigurationLocalizationV0 {
                    should_capitalize: true,
                    singular_form: "ok".to_string(),
                    plural_form: "oks".to_string(),
                }),
            ); // invalid

            contract.set_tokens(BTreeMap::from([(0, token)]));

            let transition = make_update_transition(contract, identity_nonce, platform_version);
            let mut context =
                StateTransitionExecutionContext::default_for_platform_version(platform_version)
                    .unwrap();

            let result = transition
                .validate_advanced_structure_v0(&mut context, platform_version)
                .unwrap();

            assert!(result.errors.iter().any(|e| matches!(
                e,
                ConsensusError::BasicError(BasicError::MissingDefaultLocalizationError(_))
            )));
        }

        #[test]
        fn should_succeed_with_valid_contract() {
            let platform_version = PlatformVersion::latest();
            let identity_nonce = 999;

            let mut contract =
                get_data_contract_fixture(None, identity_nonce, platform_version.protocol_version)
                    .data_contract_owned();

            contract.set_version(2); // required for update

            let transition = make_update_transition(contract, identity_nonce, platform_version);
            let mut context =
                StateTransitionExecutionContext::default_for_platform_version(platform_version)
                    .unwrap();

            let result = transition
                .validate_advanced_structure_v0(&mut context, platform_version)
                .unwrap();

            assert!(result.is_valid());
            assert_eq!(result.errors.len(), 0);
        }

        // #[test]
        // fn should_fail_if_contract_version_is_initial() {
        //     let platform_version = PlatformVersion::latest();
        //     let identity_nonce = 123;

        //     let mut contract =
        //         get_data_contract_fixture(None, identity_nonce, platform_version.protocol_version)
        //             .data_contract_owned();

        //     contract.set_version(1); // still initial

        //     let transition = make_update_transition(contract, identity_nonce, platform_version);

        //     let mut context =
        //         StateTransitionExecutionContext::default_for_platform_version(platform_version)
        //             .unwrap();
        //     let result = transition
        //         .validate_advanced_structure_v0(&mut context, platform_version)
        //         .unwrap();

        //     assert_matches!(
        //         result.errors.as_slice(),
        //         [ConsensusError::BasicError(BasicError::InvalidDataContractVersionError(e))]
        //         if e.version() == 1 && e.expected_version() == 2
        //     );

        //     assert_matches!(
        //         result.data,
        //         Some(StateTransitionAction::BumpIdentityDataContractNonceAction(action))
        //         if action.identity_contract_nonce() == identity_nonce
        //     );
        // }

        // #[test]
        // fn should_fail_if_id_does_not_match_nonce() {
        //     let platform_version = PlatformVersion::latest();
        //     let identity_nonce = 456;

        //     let mut contract =
        //         get_data_contract_fixture(None, identity_nonce, platform_version.protocol_version)
        //             .data_contract_owned();

        //     contract.set_id(Identifier::default()); // force mismatch

        //     let transition = make_update_transition(contract, identity_nonce, platform_version);

        //     let mut context =
        //         StateTransitionExecutionContext::default_for_platform_version(platform_version)
        //             .unwrap();
        //     let result = transition
        //         .validate_advanced_structure_v0(&mut context, platform_version)
        //         .unwrap();

        //     assert_matches!(
        //         result.errors.as_slice(),
        //         [ConsensusError::BasicError(
        //             BasicError::InvalidDataContractIdError(_)
        //         )]
        //     );

        //     assert_matches!(
        //         result.data,
        //         Some(StateTransitionAction::BumpIdentityDataContractNonceAction(
        //             _
        //         ))
        //     );
        // }

        // #[test]
        // fn should_fail_if_token_positions_are_non_contiguous() {
        //     let platform_version = PlatformVersion::latest();
        //     let identity_nonce = 789;

        //     let mut contract =
        //         get_data_contract_fixture(None, identity_nonce, platform_version.protocol_version)
        //             .data_contract_owned();

        //     // Add 2 tokens but give them wrong positions
        //     contract.set_tokens(BTreeMap::from([
        //         (
        //             0,
        //             TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive()),
        //         ),
        //         (
        //             1,
        //             TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive()),
        //         ),
        //     ]));

        //     let transition = make_update_transition(contract, identity_nonce, platform_version);
        //     let mut context =
        //         StateTransitionExecutionContext::default_for_platform_version(platform_version)
        //             .unwrap();

        //     let result = transition
        //         .validate_advanced_structure_v0(&mut context, platform_version)
        //         .unwrap();

        //     assert_matches!(
        //         result.errors.as_slice(),
        //         [ConsensusError::BasicError(
        //             BasicError::NonContiguousContractTokenPositionsError(_)
        //         )]
        //     );
        // }

        // #[test]
        // fn should_fail_if_token_group_does_not_exist() {
        //     let platform_version = PlatformVersion::latest();
        //     let identity_nonce = 987;

        //     let mut contract =
        //         get_data_contract_fixture(None, identity_nonce, platform_version.protocol_version)
        //             .data_contract_owned();

        //     let token = TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive());
        //     contract.set_tokens(BTreeMap::from([(0, token)]));

        //     contract.set_groups(BTreeMap::new()); // no groups defined

        //     let transition = make_update_transition(contract, identity_nonce, platform_version);
        //     let mut context =
        //         StateTransitionExecutionContext::default_for_platform_version(platform_version)
        //             .unwrap();

        //     let result = transition
        //         .validate_advanced_structure_v0(&mut context, platform_version)
        //         .unwrap();

        //     assert!(result.errors.iter().any(|e| matches!(
        //         e,
        //         ConsensusError::BasicError(BasicError::GroupExceedsMaxMembersError(_))
        //     )));
        // }
    }
}
