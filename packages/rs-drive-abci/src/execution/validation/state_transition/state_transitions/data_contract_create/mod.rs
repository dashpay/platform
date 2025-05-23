mod advanced_structure;
mod basic_structure;
mod identity_nonce;
mod state;

use advanced_structure::v1::DataContractCreatedStateTransitionAdvancedStructureValidationV1;
use basic_structure::v0::DataContractCreateStateTransitionBasicStructureValidationV0;
use dpp::block::block_info::BlockInfo;
use dpp::identity::PartialIdentity;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

use drive::grovedb::TransactionArg;
use drive::state_transition_action::StateTransitionAction;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;

use crate::execution::validation::state_transition::data_contract_create::advanced_structure::v0::DataContractCreatedStateTransitionAdvancedStructureValidationV0;
use crate::execution::validation::state_transition::data_contract_create::state::v0::DataContractCreateStateTransitionStateValidationV0;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

use crate::execution::validation::state_transition::processor::v0::{
    StateTransitionAdvancedStructureValidationV0, StateTransitionBasicStructureValidationV0,
    StateTransitionStateValidationV0,
};
use crate::execution::validation::state_transition::transformer::StateTransitionActionTransformerV0;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;

impl ValidationMode {
    /// Returns if we should validate the contract when we transform it from its serialized form
    pub fn should_fully_validate_contract_on_transform_into_action(&self) -> bool {
        match self {
            ValidationMode::CheckTx => false,
            ValidationMode::RecheckTx => false,
            ValidationMode::Validator => true,
            ValidationMode::NoValidation => false,
        }
    }
}

impl StateTransitionActionTransformerV0 for DataContractCreateTransition {
    fn transform_into_action<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        _tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;

        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .contract_create_state_transition
            .transform_into_action
        {
            0 => self.transform_into_action_v0::<C>(
                block_info,
                validation_mode,
                execution_context,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "data contract create transition: transform_into_action".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl StateTransitionBasicStructureValidationV0 for DataContractCreateTransition {
    fn validate_basic_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .contract_create_state_transition
            .basic_structure
        {
            Some(0) => self.validate_basic_structure_v0(platform_version),
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "data contract create transition: validate_basic_structure".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Execution(ExecutionError::VersionNotActive {
                method: "data contract create transition: validate_basic_structure".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}

impl StateTransitionAdvancedStructureValidationV0 for DataContractCreateTransition {
    fn validate_advanced_structure(
        &self,
        _identity: &PartialIdentity,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .contract_create_state_transition
            .advanced_structure
        {
            Some(0) => self.validate_advanced_structure_v0(execution_context),
            Some(1) => self.validate_advanced_structure_v1(execution_context),
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "data contract create transition: validate_advanced_structure".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
            None => Err(Error::Execution(ExecutionError::VersionNotActive {
                method: "data contract create transition: validate_advanced_structure".to_string(),
                known_versions: vec![0, 1],
            })),
        }
    }

    fn has_advanced_structure_validation_without_state(&self) -> bool {
        true
    }
}

impl StateTransitionStateValidationV0 for DataContractCreateTransition {
    fn validate_state<C: CoreRPCLike>(
        &self,
        _action: Option<StateTransitionAction>,
        platform: &PlatformRef<C>,
        validation_mode: ValidationMode,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;

        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .contract_create_state_transition
            .state
        {
            0 => self.validate_state_v0(
                platform,
                block_info,
                validation_mode,
                tx,
                execution_context,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "data contract create transition: validate_state".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::execution::validation::state_transition::state_transitions::tests::setup_identity;
    use crate::execution::validation::state_transition::tests::create_token_contract_with_owner_identity;
    use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use assert_matches::assert_matches;
    use dpp::balances::credits::TokenAmount;
    use dpp::block::block_info::BlockInfo;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::ConsensusError;
    use dpp::dash_to_credits;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::accessors::v1::DataContractV1Getters;
    use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Setters;
    use dpp::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
    use dpp::data_contract::change_control_rules::v0::ChangeControlRulesV0;
    use dpp::data_contract::change_control_rules::ChangeControlRules;
    use dpp::data_contract::document_type::accessors::{
        DocumentTypeV0MutGetters, DocumentTypeV1Setters,
    };
    use dpp::data_contract::group::v0::GroupV0;
    use dpp::data_contract::group::Group;
    use dpp::data_contract::DataContract;
    use dpp::data_contract::TokenConfiguration;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::TimestampMillis;
    use dpp::platform_value::Value;
    use dpp::prelude::Identifier;
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::data_contract_create_transition::methods::DataContractCreateTransitionMethodsV0;
    use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
    use dpp::tests::json_document::json_document_to_contract_with_ids;
    use dpp::tokens::calculate_token_id;
    use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
    use dpp::tokens::token_amount_on_contract_token::{
        DocumentActionTokenCost, DocumentActionTokenEffect,
    };
    use platform_version::version::PlatformVersion;
    use std::collections::BTreeMap;

    #[test]
    fn test_data_contract_creation_with_additional_property() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(2.0));

        let data_contract = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/additional_properties.json",
            None,
            None,
            true,
            platform_version,
        )
        .expect("expected to get json based contract");

        let data_contract_create_transition = DataContractCreateTransition::new_from_data_contract(
            data_contract,
            1,
            &identity.into_partial_identity_info(),
            key.id(),
            &signer,
            platform_version,
            None,
        )
        .expect("expect to create documents batch transition");

        let data_contract_create_serialized_transition = data_contract_create_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[data_contract_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");
    }

    #[test]
    fn test_data_contract_creation_with_contested_unique_index() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(2.0));

        let data_contract = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/dpns/dpns-contract-contested-unique-index.json",
            None,
            None,
            false, //no need to validate the data contracts in tests for drive
            platform_version,
        )
        .expect("expected to get json based contract");

        let data_contract_create_transition = DataContractCreateTransition::new_from_data_contract(
            data_contract,
            1,
            &identity.into_partial_identity_info(),
            key.id(),
            &signer,
            platform_version,
            None,
        )
        .expect("expect to create documents batch transition");

        let data_contract_create_serialized_transition = data_contract_create_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[data_contract_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");
    }

    #[test]
    fn test_data_contract_creation_with_contested_unique_index_old_version_has_low_fees() {
        let platform_version = PlatformVersion::get(8).unwrap();
        let mut platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(8)
            .build_with_mock_rpc()
            .set_genesis_state();

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let data_contract = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/dpns/dpns-contract-contested-unique-index.json",
            None,
            None,
            false, //no need to validate the data contracts in tests for drive
            platform_version,
        )
        .expect("expected to get json based contract");

        let data_contract_create_transition = DataContractCreateTransition::new_from_data_contract(
            data_contract,
            1,
            &identity.into_partial_identity_info(),
            key.id(),
            &signer,
            platform_version,
            None,
        )
        .expect("expect to create documents batch transition");

        let data_contract_create_serialized_transition = data_contract_create_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[data_contract_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");
    }

    #[test]
    fn test_dpns_contract_creation_with_contract_id_non_contested() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(2.0));

        let data_contract = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/dpns/dpns-contract-contested-unique-index-with-contract-id.json",
            None,
            None,
            false, //no need to validate the data contracts in tests for drive
            platform_version,
        )
            .expect("expected to get json based contract");

        let data_contract_create_transition = DataContractCreateTransition::new_from_data_contract(
            data_contract,
            1,
            &identity.into_partial_identity_info(),
            key.id(),
            &signer,
            platform_version,
            None,
        )
        .expect("expect to create documents batch transition");

        let data_contract_create_serialized_transition = data_contract_create_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[data_contract_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");
    }

    #[test]
    fn test_data_contract_creation_with_contested_unique_index_and_unique_index_should_fail() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(2.0));

        let data_contract = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/dpns/dpns-contract-contested-unique-index-and-other-unique-index.json",
            None,
            None,
            false, //no need to validate the data contracts in tests for drive
            platform_version,
        )
            .expect("expected to get json based contract");

        let data_contract_create_transition = DataContractCreateTransition::new_from_data_contract(
            data_contract,
            1,
            &identity.into_partial_identity_info(),
            key.id(),
            &signer,
            platform_version,
            None,
        )
        .expect("expect to create documents batch transition");

        let data_contract_create_serialized_transition = data_contract_create_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[data_contract_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError(
                ConsensusError::BasicError(BasicError::ContestedUniqueIndexWithUniqueIndexError(_)),
                _
            )]
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");
    }
    #[cfg(test)]
    mod tokens {
        use super::*;
        use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
        use dpp::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Setters;

        mod basic_creation {
            use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::{DistributionFunction, MAX_DISTRIBUTION_PARAM};
            use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient;
            use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
            use dpp::data_contract::associated_token::token_perpetual_distribution::TokenPerpetualDistribution;
            use dpp::data_contract::associated_token::token_perpetual_distribution::v0::TokenPerpetualDistributionV0;
            use super::*;
            #[test]
            fn test_data_contract_creation_with_single_token() {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, 958, dash_to_credits!(1.0));

                let mut data_contract = json_document_to_contract_with_ids(
                    "tests/supporting_files/contract/basic-token/basic-token.json",
                    None,
                    None,
                    false, //no need to validate the data contracts in tests for drive
                    platform_version,
                )
                .expect("expected to get json based contract");

                let identity_id = identity.id();

                let base_supply_start_amount = 0;

                {
                    let token_config = data_contract
                        .tokens_mut()
                        .expect("expected tokens")
                        .get_mut(&0)
                        .expect("expected first token");
                    token_config.set_base_supply(base_supply_start_amount);
                }

                let data_contract_id = DataContract::generate_data_contract_id_v0(identity_id, 1);

                let token_id = calculate_token_id(data_contract_id.as_bytes(), 0);

                let data_contract_create_transition =
                    DataContractCreateTransition::new_from_data_contract(
                        data_contract,
                        1,
                        &identity.into_partial_identity_info(),
                        key.id(),
                        &signer,
                        platform_version,
                        None,
                    )
                    .expect("expect to create documents batch transition");

                let data_contract_create_serialized_transition = data_contract_create_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &[data_contract_create_serialized_transition.clone()],
                        &platform_state,
                        &BlockInfo::default(),
                        &transaction,
                        platform_version,
                        false,
                        None,
                    )
                    .expect("expected to process state transition");

                assert_matches!(
                    processing_result.execution_results().as_slice(),
                    [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
                );

                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .expect("expected to commit transaction");

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id,
                        identity_id.to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, None);
            }

            #[test]
            fn test_data_contract_creation_with_single_token_and_group() {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, 958, dash_to_credits!(1.0));

                let (identity_2, _, _) = setup_identity(&mut platform, 234, dash_to_credits!(0.1));

                let (identity_3, _, _) = setup_identity(&mut platform, 45, dash_to_credits!(0.1));

                let mut data_contract = json_document_to_contract_with_ids(
                    "tests/supporting_files/contract/basic-token/basic-token.json",
                    None,
                    None,
                    false, //no need to validate the data contracts in tests for drive
                    platform_version,
                )
                .expect("expected to get json based contract");

                let identity_id = identity.id();

                let base_supply_start_amount = 0;

                {
                    let groups = data_contract.groups_mut().expect("expected tokens");
                    groups.insert(
                        0,
                        Group::V0(GroupV0 {
                            members: [(identity.id(), 1), (identity_2.id(), 1)].into(),
                            required_power: 2,
                        }),
                    );
                    groups.insert(
                        1,
                        Group::V0(GroupV0 {
                            members: [
                                (identity.id(), 1),
                                (identity_3.id(), 1),
                                (identity_2.id(), 2),
                            ]
                            .into(),
                            required_power: 2,
                        }),
                    );
                    let token_config = data_contract
                        .tokens_mut()
                        .expect("expected tokens")
                        .get_mut(&0)
                        .expect("expected first token");
                    token_config.set_main_control_group(Some(1));
                    token_config.set_base_supply(base_supply_start_amount);
                    token_config.set_manual_minting_rules(ChangeControlRules::V0(
                        ChangeControlRulesV0 {
                            authorized_to_make_change: AuthorizedActionTakers::Group(0),
                            // We have no group at position 1, we should get an error
                            admin_action_takers: AuthorizedActionTakers::MainGroup,
                            changing_authorized_action_takers_to_no_one_allowed: false,
                            changing_admin_action_takers_to_no_one_allowed: false,
                            self_changing_admin_action_takers_allowed: false,
                        },
                    ));
                }

                let data_contract_id = DataContract::generate_data_contract_id_v0(identity_id, 1);

                let token_id = calculate_token_id(data_contract_id.as_bytes(), 0);

                let data_contract_create_transition =
                    DataContractCreateTransition::new_from_data_contract(
                        data_contract,
                        1,
                        &identity.into_partial_identity_info(),
                        key.id(),
                        &signer,
                        platform_version,
                        None,
                    )
                    .expect("expect to create documents batch transition");

                let data_contract_create_serialized_transition = data_contract_create_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &[data_contract_create_serialized_transition.clone()],
                        &platform_state,
                        &BlockInfo::default(),
                        &transaction,
                        platform_version,
                        false,
                        None,
                    )
                    .expect("expected to process state transition");

                assert_matches!(
                    processing_result.execution_results().as_slice(),
                    [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
                );

                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .expect("expected to commit transaction");

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id,
                        identity_id.to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, None);
            }

            #[test]
            fn test_data_contract_creation_with_single_token_with_starting_balance() {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, 958, dash_to_credits!(1.0));

                let mut data_contract = json_document_to_contract_with_ids(
                    "tests/supporting_files/contract/basic-token/basic-token.json",
                    None,
                    None,
                    false, //no need to validate the data contracts in tests for drive
                    platform_version,
                )
                .expect("expected to get json based contract");

                let base_supply_start_amount = 10000;

                {
                    let token_config = data_contract
                        .tokens_mut()
                        .expect("expected tokens")
                        .get_mut(&0)
                        .expect("expected first token");
                    token_config.set_base_supply(base_supply_start_amount);
                }

                let identity_id = identity.id();

                let data_contract_id = DataContract::generate_data_contract_id_v0(identity_id, 1);

                let data_contract_create_transition =
                    DataContractCreateTransition::new_from_data_contract(
                        data_contract,
                        1,
                        &identity.into_partial_identity_info(),
                        key.id(),
                        &signer,
                        platform_version,
                        None,
                    )
                    .expect("expect to create documents batch transition");

                let token_id = calculate_token_id(data_contract_id.as_bytes(), 0);

                let data_contract_create_serialized_transition = data_contract_create_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &[data_contract_create_serialized_transition.clone()],
                        &platform_state,
                        &BlockInfo::default(),
                        &transaction,
                        platform_version,
                        false,
                        None,
                    )
                    .expect("expected to process state transition");

                assert_matches!(
                    processing_result.execution_results().as_slice(),
                    [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
                );

                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .expect("expected to commit transaction");

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id,
                        identity_id.to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, Some(base_supply_start_amount));
            }

            #[test]
            fn test_data_contract_creation_with_single_token_setting_burn_of_internal_token_on_nft_purchase_should_be_allowed(
            ) {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let platform_state = platform.state.load();

                let (identity, contract_signer, contract_key) =
                    setup_identity(&mut platform, 958, dash_to_credits!(1.0));

                let mut data_contract = json_document_to_contract_with_ids(
                    "tests/supporting_files/contract/crypto-card-game/crypto-card-game-in-game-currency.json",
                    None,
                    None,
                    false, //no need to validate the data contracts in tests for drive
                    platform_version,
                )
                    .expect("expected to get json based contract");

                {
                    let document_type = data_contract
                        .document_types_mut()
                        .get_mut("card")
                        .expect("expected a document type with name card");
                    document_type.set_document_creation_token_cost(Some(DocumentActionTokenCost {
                        contract_id: None,
                        token_contract_position: 0,
                        token_amount: 5,
                        effect: DocumentActionTokenEffect::BurnToken,
                        gas_fees_paid_by: GasFeesPaidBy::DocumentOwner,
                    }));
                    let gas_fees_paid_by_int: u8 = GasFeesPaidBy::DocumentOwner.into();
                    let schema = document_type.schema_mut();
                    let token_cost = schema
                        .get_mut("tokenCost")
                        .expect("expected to get token cost")
                        .expect("expected token cost to be set");
                    let creation_token_cost = token_cost
                        .get_mut("create")
                        .expect("expected to get creation token cost")
                        .expect("expected creation token cost to be set");
                    creation_token_cost
                        .set_value("tokenPosition", 0.into())
                        .expect("expected to set token position");
                    creation_token_cost
                        .set_value("amount", 5.into())
                        .expect("expected to set token amount");
                    creation_token_cost
                        .set_value(
                            "effect",
                            Value::U8(DocumentActionTokenEffect::BurnToken.into()),
                        )
                        .expect("expected to set token pay effect");
                    creation_token_cost
                        .set_value("gasFeesPaidBy", gas_fees_paid_by_int.into())
                        .expect("expected to set token amount");
                }

                let data_contract_create_transition =
                    DataContractCreateTransition::new_from_data_contract(
                        data_contract,
                        1,
                        &identity.into_partial_identity_info(),
                        contract_key.id(),
                        &contract_signer,
                        platform_version,
                        None,
                    )
                    .expect("expect to create data contract create batch transition");

                let data_contract_create_serialized_transition = data_contract_create_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &[data_contract_create_serialized_transition.clone()],
                        &platform_state,
                        &BlockInfo::default(),
                        &transaction,
                        platform_version,
                        false,
                        None,
                    )
                    .expect("expected to process state transition");

                assert_matches!(
                    processing_result.execution_results().as_slice(),
                    [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
                );

                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .expect("expected to commit transaction");
            }

            #[test]
            fn test_data_contract_creation_with_single_token_setting_transfer_on_nft_purchase_with_internal_token_should_be_allowed(
            ) {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let platform_state = platform.state.load();

                let (identity, contract_signer, contract_key) =
                    setup_identity(&mut platform, 958, dash_to_credits!(1.0));

                let mut data_contract = json_document_to_contract_with_ids(
                    "tests/supporting_files/contract/crypto-card-game/crypto-card-game-in-game-currency.json",
                    None,
                    None,
                    false, //no need to validate the data contracts in tests for drive
                    platform_version,
                )
                    .expect("expected to get json based contract");

                {
                    let document_type = data_contract
                        .document_types_mut()
                        .get_mut("card")
                        .expect("expected a document type with name card");
                    document_type.set_document_creation_token_cost(Some(DocumentActionTokenCost {
                        contract_id: None,
                        token_contract_position: 0,
                        token_amount: 5,
                        effect: DocumentActionTokenEffect::TransferTokenToContractOwner,
                        gas_fees_paid_by: GasFeesPaidBy::DocumentOwner,
                    }));
                    let gas_fees_paid_by_int: u8 = GasFeesPaidBy::DocumentOwner.into();
                    let schema = document_type.schema_mut();
                    let token_cost = schema
                        .get_mut("tokenCost")
                        .expect("expected to get token cost")
                        .expect("expected token cost to be set");
                    let creation_token_cost = token_cost
                        .get_mut("create")
                        .expect("expected to get creation token cost")
                        .expect("expected creation token cost to be set");
                    creation_token_cost
                        .set_value("tokenPosition", 0.into())
                        .expect("expected to set token position");
                    creation_token_cost
                        .set_value("amount", 5.into())
                        .expect("expected to set token amount");
                    creation_token_cost
                        .set_value(
                            "effect",
                            Value::U8(
                                DocumentActionTokenEffect::TransferTokenToContractOwner.into(),
                            ),
                        )
                        .expect("expected to set token pay effect");
                    creation_token_cost
                        .set_value("gasFeesPaidBy", gas_fees_paid_by_int.into())
                        .expect("expected to set token amount");
                }

                let data_contract_create_transition =
                    DataContractCreateTransition::new_from_data_contract(
                        data_contract,
                        1,
                        &identity.into_partial_identity_info(),
                        contract_key.id(),
                        &contract_signer,
                        platform_version,
                        None,
                    )
                    .expect("expect to create data contract create batch transition");

                let data_contract_create_serialized_transition = data_contract_create_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &[data_contract_create_serialized_transition.clone()],
                        &platform_state,
                        &BlockInfo::default(),
                        &transaction,
                        platform_version,
                        false,
                        None,
                    )
                    .expect("expected to process state transition");

                assert_matches!(
                    processing_result.execution_results().as_slice(),
                    [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
                );

                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .expect("expected to commit transaction");
            }

            #[test]
            fn test_data_contract_creation_with_single_token_setting_identifier_that_does_exist() {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, 958, dash_to_credits!(1.0));

                let (identity_2, _signer_2, _key_2) =
                    setup_identity(&mut platform, 93, dash_to_credits!(0.5));

                let mut data_contract = json_document_to_contract_with_ids(
                    "tests/supporting_files/contract/basic-token/basic-token.json",
                    None,
                    None,
                    false, //no need to validate the data contracts in tests for drive
                    platform_version,
                )
                .expect("expected to get json based contract");

                let identity_id = identity.id();

                {
                    let token_config = data_contract
                        .tokens_mut()
                        .expect("expected tokens")
                        .get_mut(&0)
                        .expect("expected first token");
                    token_config.set_manual_minting_rules(ChangeControlRules::V0(
                        ChangeControlRulesV0 {
                            authorized_to_make_change: AuthorizedActionTakers::Identity(
                                identity_2.id(),
                            ),
                            // We have no group at position 1, we should get an error
                            admin_action_takers: AuthorizedActionTakers::ContractOwner,
                            changing_authorized_action_takers_to_no_one_allowed: false,
                            changing_admin_action_takers_to_no_one_allowed: false,
                            self_changing_admin_action_takers_allowed: false,
                        },
                    ));
                }

                let data_contract_id = DataContract::generate_data_contract_id_v0(identity_id, 1);

                let token_id = calculate_token_id(data_contract_id.as_bytes(), 0);

                let data_contract_create_transition =
                    DataContractCreateTransition::new_from_data_contract(
                        data_contract,
                        1,
                        &identity.into_partial_identity_info(),
                        key.id(),
                        &signer,
                        platform_version,
                        None,
                    )
                    .expect("expect to create documents batch transition");

                let data_contract_create_serialized_transition = data_contract_create_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &[data_contract_create_serialized_transition.clone()],
                        &platform_state,
                        &BlockInfo::default(),
                        &transaction,
                        platform_version,
                        false,
                        None,
                    )
                    .expect("expected to process state transition");

                assert_matches!(
                    processing_result.execution_results().as_slice(),
                    [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
                );

                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .expect("expected to commit transaction");

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id,
                        identity_id.to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, Some(100_000));
            }

            #[test]
            fn test_data_contract_creation_with_single_token_setting_transfer_on_nft_purchase_with_external_token_should_be_allowed(
            ) {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let platform_state = platform.state.load();

                let (identity, contract_signer, contract_key) =
                    setup_identity(&mut platform, 958, dash_to_credits!(1.0));

                let (token_contract_owner_id, _, _) =
                    setup_identity(&mut platform, 11, dash_to_credits!(0.1));

                let (token_contract, _) = create_token_contract_with_owner_identity(
                    &mut platform,
                    token_contract_owner_id.id(),
                    None::<fn(&mut TokenConfiguration)>,
                    None,
                    None,
                    platform_version,
                );

                let token_contract_id = token_contract.id();

                let mut data_contract = json_document_to_contract_with_ids(
                    "tests/supporting_files/contract/crypto-card-game/crypto-card-game-in-game-currency.json",
                    None,
                    None,
                    false, //no need to validate the data contracts in tests for drive
                    platform_version,
                )
                    .expect("expected to get json based contract");

                {
                    let document_type = data_contract
                        .document_types_mut()
                        .get_mut("card")
                        .expect("expected a document type with name card");
                    document_type.set_document_creation_token_cost(Some(DocumentActionTokenCost {
                        contract_id: Some(token_contract_id),
                        token_contract_position: 0,
                        token_amount: 5,
                        effect: DocumentActionTokenEffect::TransferTokenToContractOwner,
                        gas_fees_paid_by: GasFeesPaidBy::DocumentOwner,
                    }));
                    let gas_fees_paid_by_int: u8 = GasFeesPaidBy::DocumentOwner.into();
                    let schema = document_type.schema_mut();
                    let token_cost = schema
                        .get_mut("tokenCost")
                        .expect("expected to get token cost")
                        .expect("expected token cost to be set");
                    let creation_token_cost = token_cost
                        .get_mut("create")
                        .expect("expected to get creation token cost")
                        .expect("expected creation token cost to be set");
                    creation_token_cost
                        .set_value("contractId", token_contract_id.into())
                        .expect("expected to set token contract id");
                    creation_token_cost
                        .set_value("tokenPosition", 0.into())
                        .expect("expected to set token position");
                    creation_token_cost
                        .set_value("amount", 5.into())
                        .expect("expected to set token amount");
                    creation_token_cost
                        .set_value(
                            "effect",
                            Value::U8(
                                DocumentActionTokenEffect::TransferTokenToContractOwner.into(),
                            ),
                        )
                        .expect("expected to set token pay effect");
                    creation_token_cost
                        .set_value("gasFeesPaidBy", gas_fees_paid_by_int.into())
                        .expect("expected to set token amount");
                }

                let data_contract_create_transition =
                    DataContractCreateTransition::new_from_data_contract(
                        data_contract,
                        1,
                        &identity.into_partial_identity_info(),
                        contract_key.id(),
                        &contract_signer,
                        platform_version,
                        None,
                    )
                    .expect("expect to create data contract create batch transition");

                let data_contract_create_serialized_transition = data_contract_create_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &[data_contract_create_serialized_transition.clone()],
                        &platform_state,
                        &BlockInfo::default(),
                        &transaction,
                        platform_version,
                        false,
                        None,
                    )
                    .expect("expected to process state transition");

                assert_matches!(
                    processing_result.execution_results().as_slice(),
                    [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
                );

                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .expect("expected to commit transaction");
            }

            #[test]
            fn test_data_contract_creation_with_single_token_with_valid_perpetual_distribution() {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, 958, dash_to_credits!(1.0));

                let mut data_contract = json_document_to_contract_with_ids(
                    "tests/supporting_files/contract/basic-token/basic-token.json",
                    None,
                    None,
                    false, //no need to validate the data contracts in tests for drive
                    platform_version,
                )
                .expect("expected to get json based contract");

                {
                    let token_config = data_contract
                        .tokens_mut()
                        .expect("expected tokens")
                        .get_mut(&0)
                        .expect("expected first token");
                    token_config
                        .distribution_rules_mut()
                        .set_perpetual_distribution(Some(TokenPerpetualDistribution::V0(
                            TokenPerpetualDistributionV0 {
                                distribution_type: RewardDistributionType::BlockBasedDistribution {
                                    interval: 10,
                                    function: DistributionFunction::Exponential {
                                        a: 1,
                                        d: 1,
                                        m: 1,
                                        n: 1,
                                        o: 0,
                                        start_moment: None,
                                        b: 10,
                                        min_value: None,
                                        max_value: Some(MAX_DISTRIBUTION_PARAM),
                                    },
                                },
                                // we give to identity 2
                                distribution_recipient: TokenDistributionRecipient::Identity(
                                    identity.id(),
                                ),
                            },
                        )));
                }

                let identity_id = identity.id();

                let data_contract_id = DataContract::generate_data_contract_id_v0(identity_id, 1);

                let data_contract_create_transition =
                    DataContractCreateTransition::new_from_data_contract(
                        data_contract,
                        1,
                        &identity.into_partial_identity_info(),
                        key.id(),
                        &signer,
                        platform_version,
                        None,
                    )
                    .expect("expect to create documents batch transition");

                let token_id = calculate_token_id(data_contract_id.as_bytes(), 0);

                let data_contract_create_serialized_transition = data_contract_create_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &[data_contract_create_serialized_transition.clone()],
                        &platform_state,
                        &BlockInfo::default(),
                        &transaction,
                        platform_version,
                        false,
                        None,
                    )
                    .expect("expected to process state transition");
                assert_matches!(
                    processing_result.execution_results().as_slice(),
                    [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
                );

                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .expect("expected to commit transaction");

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id,
                        identity_id.to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, Some(100_000));
            }
        }

        mod pre_programmed_distribution {
            use super::*;
            use dpp::data_contract::associated_token::token_pre_programmed_distribution::v0::TokenPreProgrammedDistributionV0;
            use dpp::data_contract::associated_token::token_pre_programmed_distribution::TokenPreProgrammedDistribution;
            use drive::drive::Drive;

            #[test]
            fn test_data_contract_pre_programmed_distribution() {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, 958, dash_to_credits!(1.0));

                let (identity_2, _, _) = setup_identity(&mut platform, 5456, dash_to_credits!(0.1));

                let (identity_3, _, _) = setup_identity(&mut platform, 123, dash_to_credits!(0.1));

                let (identity_4, _, _) = setup_identity(&mut platform, 548, dash_to_credits!(0.1));

                let (identity_5, _, _) = setup_identity(&mut platform, 467, dash_to_credits!(0.1));

                let mut data_contract = json_document_to_contract_with_ids(
                    "tests/supporting_files/contract/basic-token/basic-token.json",
                    None,
                    None,
                    false, //no need to validate the data contracts in tests for drive
                    platform_version,
                )
                .expect("expected to get json based contract");

                let identity_id = identity.id();

                let base_supply_start_amount = 0;

                let token_config = data_contract
                    .tokens_mut()
                    .expect("expected tokens")
                    .get_mut(&0)
                    .expect("expected first token");
                token_config.set_base_supply(base_supply_start_amount);

                // Create a new BTreeMap to store distributions
                let mut distributions: BTreeMap<
                    TimestampMillis,
                    BTreeMap<Identifier, TokenAmount>,
                > = BTreeMap::new();

                // Create distributions for different timestamps
                distributions.insert(
                    1700000000000, // Example timestamp (milliseconds)
                    BTreeMap::from([
                        (identity.id(), 10),  // Identity 1 gets 10 tokens
                        (identity_2.id(), 5), // Identity 2 gets 5 tokens
                    ]),
                );

                distributions.insert(
                    1700005000000, // Another timestamp
                    BTreeMap::from([
                        (identity_3.id(), 15), // Identity 3 gets 15 tokens
                        (identity_4.id(), 20), // Identity 4 gets 20 tokens
                        (identity_5.id(), 25), // Identity 5 gets 25 tokens
                    ]),
                );

                token_config
                    .distribution_rules_mut()
                    .set_pre_programmed_distribution(Some(TokenPreProgrammedDistribution::V0(
                        TokenPreProgrammedDistributionV0 {
                            distributions: distributions.clone(),
                        },
                    )));

                let data_contract_id = DataContract::generate_data_contract_id_v0(identity_id, 1);

                let token_id = calculate_token_id(data_contract_id.as_bytes(), 0);

                let data_contract_create_transition =
                    DataContractCreateTransition::new_from_data_contract(
                        data_contract,
                        1,
                        &identity.into_partial_identity_info(),
                        key.id(),
                        &signer,
                        platform_version,
                        None,
                    )
                    .expect("expect to create documents batch transition");

                let data_contract_create_serialized_transition = data_contract_create_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &[data_contract_create_serialized_transition.clone()],
                        &platform_state,
                        &BlockInfo::default(),
                        &transaction,
                        platform_version,
                        false,
                        None,
                    )
                    .expect("expected to process state transition");

                assert_matches!(
                    processing_result.execution_results().as_slice(),
                    [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
                );

                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .expect("expected to commit transaction");

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id,
                        identity_id.to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, None);

                let fetched_distributions = platform
                    .drive
                    .fetch_token_pre_programmed_distributions(
                        token_id,
                        None,
                        None,
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch pre-programmed distributions");

                assert_eq!(fetched_distributions, distributions);

                let proved_distributions = platform
                    .drive
                    .prove_token_pre_programmed_distributions(
                        token_id,
                        None,
                        None,
                        None,
                        platform_version,
                    )
                    .expect("expected to prove pre-programmed distributions");

                let verified_pre_programmed_distributions: BTreeMap<
                    TimestampMillis,
                    BTreeMap<Identifier, TokenAmount>,
                > = Drive::verify_token_pre_programmed_distributions(
                    proved_distributions.as_slice(),
                    token_id,
                    None,
                    None,
                    false,
                    platform_version,
                )
                .expect("expected to verify proof")
                .1;

                assert_eq!(verified_pre_programmed_distributions, distributions);
            }
        }

        mod token_errors {
            use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
            use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient;
            use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
            use dpp::data_contract::associated_token::token_perpetual_distribution::TokenPerpetualDistribution;
            use dpp::data_contract::associated_token::token_perpetual_distribution::v0::TokenPerpetualDistributionV0;
            use super::*;
            use dpp::consensus::state::state_error::StateError;
            use dpp::data_contract::associated_token::token_pre_programmed_distribution::TokenPreProgrammedDistribution;
            use dpp::data_contract::associated_token::token_pre_programmed_distribution::v0::TokenPreProgrammedDistributionV0;

            #[test]
            fn test_data_contract_creation_with_single_token_with_starting_balance_over_limit_should_cause_error(
            ) {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, 958, dash_to_credits!(1.0));

                let mut data_contract = json_document_to_contract_with_ids(
                    "tests/supporting_files/contract/basic-token/basic-token.json",
                    None,
                    None,
                    false, //no need to validate the data contracts in tests for drive
                    platform_version,
                )
                .expect("expected to get json based contract");

                let base_supply_start_amount = u64::MAX;

                {
                    let token_config = data_contract
                        .tokens_mut()
                        .expect("expected tokens")
                        .get_mut(&0)
                        .expect("expected first token");
                    token_config.set_base_supply(base_supply_start_amount);
                }

                let identity_id = identity.id();

                let data_contract_id = DataContract::generate_data_contract_id_v0(identity_id, 1);

                let data_contract_create_transition =
                    DataContractCreateTransition::new_from_data_contract(
                        data_contract,
                        1,
                        &identity.into_partial_identity_info(),
                        key.id(),
                        &signer,
                        platform_version,
                        None,
                    )
                    .expect("expect to create documents batch transition");

                let token_id = calculate_token_id(data_contract_id.as_bytes(), 0);

                let data_contract_create_serialized_transition = data_contract_create_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &[data_contract_create_serialized_transition.clone()],
                        &platform_state,
                        &BlockInfo::default(),
                        &transaction,
                        platform_version,
                        false,
                        None,
                    )
                    .expect("expected to process state transition");
                assert_matches!(
                    processing_result.execution_results().as_slice(),
                    [StateTransitionExecutionResult::UnpaidConsensusError(
                        ConsensusError::BasicError(BasicError::InvalidTokenBaseSupplyError(_)),
                    )]
                );

                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .expect("expected to commit transaction");

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id,
                        identity_id.to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, None);
            }

            #[test]
            fn test_data_contract_creation_with_single_token_needing_group_that_does_not_exist() {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, 958, dash_to_credits!(1.0));

                let (identity_2, _, _) = setup_identity(&mut platform, 564, dash_to_credits!(0.1));

                let mut data_contract = json_document_to_contract_with_ids(
                    "tests/supporting_files/contract/basic-token/basic-token.json",
                    None,
                    None,
                    false, //no need to validate the data contracts in tests for drive
                    platform_version,
                )
                .expect("expected to get json based contract");

                let identity_id = identity.id();

                {
                    let groups = data_contract.groups_mut().expect("expected tokens");
                    groups.insert(
                        0,
                        Group::V0(GroupV0 {
                            members: [(identity.id(), 1), (identity_2.id(), 1)].into(),
                            required_power: 2,
                        }),
                    );
                    let token_config = data_contract
                        .tokens_mut()
                        .expect("expected tokens")
                        .get_mut(&0)
                        .expect("expected first token");
                    token_config.set_manual_minting_rules(ChangeControlRules::V0(
                        ChangeControlRulesV0 {
                            authorized_to_make_change: AuthorizedActionTakers::Group(0),
                            // We have no group at position 1, we should get an error
                            admin_action_takers: AuthorizedActionTakers::Group(1),
                            changing_authorized_action_takers_to_no_one_allowed: false,
                            changing_admin_action_takers_to_no_one_allowed: false,
                            self_changing_admin_action_takers_allowed: false,
                        },
                    ));
                }

                let data_contract_id = DataContract::generate_data_contract_id_v0(identity_id, 1);

                let token_id = calculate_token_id(data_contract_id.as_bytes(), 0);

                let data_contract_create_transition =
                    DataContractCreateTransition::new_from_data_contract(
                        data_contract,
                        1,
                        &identity.into_partial_identity_info(),
                        key.id(),
                        &signer,
                        platform_version,
                        None,
                    )
                    .expect("expect to create documents batch transition");

                let data_contract_create_serialized_transition = data_contract_create_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &[data_contract_create_serialized_transition.clone()],
                        &platform_state,
                        &BlockInfo::default(),
                        &transaction,
                        platform_version,
                        false,
                        None,
                    )
                    .expect("expected to process state transition");

                assert_matches!(
                    processing_result.execution_results().as_slice(),
                    [StateTransitionExecutionResult::UnpaidConsensusError(
                        ConsensusError::BasicError(BasicError::GroupPositionDoesNotExistError(_)),
                    )]
                );

                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .expect("expected to commit transaction");

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id,
                        identity_id.to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, None);
            }

            #[test]
            fn test_data_contract_creation_with_single_token_setting_main_group_that_does_not_exist(
            ) {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, 958, dash_to_credits!(1.0));

                let (identity_2, _, _) = setup_identity(&mut platform, 564, dash_to_credits!(0.1));

                let mut data_contract = json_document_to_contract_with_ids(
                    "tests/supporting_files/contract/basic-token/basic-token.json",
                    None,
                    None,
                    false, //no need to validate the data contracts in tests for drive
                    platform_version,
                )
                .expect("expected to get json based contract");

                let identity_id = identity.id();

                {
                    let groups = data_contract.groups_mut().expect("expected tokens");
                    groups.insert(
                        0,
                        Group::V0(GroupV0 {
                            members: [(identity.id(), 1), (identity_2.id(), 1)].into(),
                            required_power: 2,
                        }),
                    );
                    let token_config = data_contract
                        .tokens_mut()
                        .expect("expected tokens")
                        .get_mut(&0)
                        .expect("expected first token");
                    token_config.set_main_control_group(Some(1));
                    token_config.set_manual_minting_rules(ChangeControlRules::V0(
                        ChangeControlRulesV0 {
                            authorized_to_make_change: AuthorizedActionTakers::Group(0),
                            // We have no group at position 1, we should get an error
                            admin_action_takers: AuthorizedActionTakers::MainGroup,
                            changing_authorized_action_takers_to_no_one_allowed: false,
                            changing_admin_action_takers_to_no_one_allowed: false,
                            self_changing_admin_action_takers_allowed: false,
                        },
                    ));
                }

                let data_contract_id = DataContract::generate_data_contract_id_v0(identity_id, 1);

                let token_id = calculate_token_id(data_contract_id.as_bytes(), 0);

                let data_contract_create_transition =
                    DataContractCreateTransition::new_from_data_contract(
                        data_contract,
                        1,
                        &identity.into_partial_identity_info(),
                        key.id(),
                        &signer,
                        platform_version,
                        None,
                    )
                    .expect("expect to create documents batch transition");

                let data_contract_create_serialized_transition = data_contract_create_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &[data_contract_create_serialized_transition.clone()],
                        &platform_state,
                        &BlockInfo::default(),
                        &transaction,
                        platform_version,
                        false,
                        None,
                    )
                    .expect("expected to process state transition");

                assert_matches!(
                    processing_result.execution_results().as_slice(),
                    [StateTransitionExecutionResult::UnpaidConsensusError(
                        ConsensusError::BasicError(BasicError::GroupPositionDoesNotExistError(_)),
                    )]
                );

                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .expect("expected to commit transaction");

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id,
                        identity_id.to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, None);
            }

            #[test]
            fn test_data_contract_creation_with_single_token_setting_authorization_to_non_defined_main_group(
            ) {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, 958, dash_to_credits!(1.0));

                let (identity_2, _, _) = setup_identity(&mut platform, 564, dash_to_credits!(0.1));

                let mut data_contract = json_document_to_contract_with_ids(
                    "tests/supporting_files/contract/basic-token/basic-token.json",
                    None,
                    None,
                    false, //no need to validate the data contracts in tests for drive
                    platform_version,
                )
                .expect("expected to get json based contract");

                let identity_id = identity.id();

                {
                    let groups = data_contract.groups_mut().expect("expected tokens");
                    groups.insert(
                        0,
                        Group::V0(GroupV0 {
                            members: [(identity.id(), 1), (identity_2.id(), 1)].into(),
                            required_power: 2,
                        }),
                    );
                    let token_config = data_contract
                        .tokens_mut()
                        .expect("expected tokens")
                        .get_mut(&0)
                        .expect("expected first token");
                    token_config.set_manual_minting_rules(ChangeControlRules::V0(
                        ChangeControlRulesV0 {
                            authorized_to_make_change: AuthorizedActionTakers::MainGroup,
                            // We have no group at position 1, we should get an error
                            admin_action_takers: AuthorizedActionTakers::MainGroup,
                            changing_authorized_action_takers_to_no_one_allowed: false,
                            changing_admin_action_takers_to_no_one_allowed: false,
                            self_changing_admin_action_takers_allowed: false,
                        },
                    ));
                }

                let data_contract_id = DataContract::generate_data_contract_id_v0(identity_id, 1);

                let token_id = calculate_token_id(data_contract_id.as_bytes(), 0);

                let data_contract_create_transition =
                    DataContractCreateTransition::new_from_data_contract(
                        data_contract,
                        1,
                        &identity.into_partial_identity_info(),
                        key.id(),
                        &signer,
                        platform_version,
                        None,
                    )
                    .expect("expect to create documents batch transition");

                let data_contract_create_serialized_transition = data_contract_create_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &[data_contract_create_serialized_transition.clone()],
                        &platform_state,
                        &BlockInfo::default(),
                        &transaction,
                        platform_version,
                        false,
                        None,
                    )
                    .expect("expected to process state transition");

                assert_matches!(
                    processing_result.execution_results().as_slice(),
                    [StateTransitionExecutionResult::UnpaidConsensusError(
                        ConsensusError::BasicError(BasicError::MainGroupIsNotDefinedError(_)),
                    )]
                );

                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .expect("expected to commit transaction");

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id,
                        identity_id.to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, None);
            }

            #[test]
            fn test_data_contract_creation_with_single_token_setting_identifier_that_does_not_exist(
            ) {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, 958, dash_to_credits!(1.0));

                let mut data_contract = json_document_to_contract_with_ids(
                    "tests/supporting_files/contract/basic-token/basic-token.json",
                    None,
                    None,
                    false, //no need to validate the data contracts in tests for drive
                    platform_version,
                )
                .expect("expected to get json based contract");

                let identity_id = identity.id();

                {
                    let token_config = data_contract
                        .tokens_mut()
                        .expect("expected tokens")
                        .get_mut(&0)
                        .expect("expected first token");
                    token_config.set_manual_minting_rules(ChangeControlRules::V0(
                        ChangeControlRulesV0 {
                            authorized_to_make_change: AuthorizedActionTakers::Identity(
                                Identifier::from([4; 32]),
                            ),
                            // We have no group at position 1, we should get an error
                            admin_action_takers: AuthorizedActionTakers::ContractOwner,
                            changing_authorized_action_takers_to_no_one_allowed: false,
                            changing_admin_action_takers_to_no_one_allowed: false,
                            self_changing_admin_action_takers_allowed: false,
                        },
                    ));
                }

                let data_contract_id = DataContract::generate_data_contract_id_v0(identity_id, 1);

                let token_id = calculate_token_id(data_contract_id.as_bytes(), 0);

                let data_contract_create_transition =
                    DataContractCreateTransition::new_from_data_contract(
                        data_contract,
                        1,
                        &identity.into_partial_identity_info(),
                        key.id(),
                        &signer,
                        platform_version,
                        None,
                    )
                    .expect("expect to create documents batch transition");

                let data_contract_create_serialized_transition = data_contract_create_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &[data_contract_create_serialized_transition.clone()],
                        &platform_state,
                        &BlockInfo::default(),
                        &transaction,
                        platform_version,
                        false,
                        None,
                    )
                    .expect("expected to process state transition");

                assert_matches!(
                    processing_result.execution_results().as_slice(),
                    [StateTransitionExecutionResult::PaidConsensusError(
                        ConsensusError::StateError(
                            StateError::IdentityInTokenConfigurationNotFoundError(_)
                        ),
                        _
                    )]
                );

                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .expect("expected to commit transaction");

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id,
                        identity_id.to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, None);
            }

            #[test]
            fn test_data_contract_creation_with_single_token_setting_minting_recipient_to_identity_that_does_not_exist(
            ) {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, 958, dash_to_credits!(1.0));

                let mut data_contract = json_document_to_contract_with_ids(
                    "tests/supporting_files/contract/basic-token/basic-token.json",
                    None,
                    None,
                    false, //no need to validate the data contracts in tests for drive
                    platform_version,
                )
                .expect("expected to get json based contract");

                let identity_id = identity.id();

                {
                    let token_config = data_contract
                        .tokens_mut()
                        .expect("expected tokens")
                        .get_mut(&0)
                        .expect("expected first token");
                    token_config
                        .distribution_rules_mut()
                        .set_new_tokens_destination_identity(Some(Identifier::from([4; 32])));
                }

                let data_contract_id = DataContract::generate_data_contract_id_v0(identity_id, 1);

                let token_id = calculate_token_id(data_contract_id.as_bytes(), 0);

                let data_contract_create_transition =
                    DataContractCreateTransition::new_from_data_contract(
                        data_contract,
                        1,
                        &identity.into_partial_identity_info(),
                        key.id(),
                        &signer,
                        platform_version,
                        None,
                    )
                    .expect("expect to create documents batch transition");

                let data_contract_create_serialized_transition = data_contract_create_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &[data_contract_create_serialized_transition.clone()],
                        &platform_state,
                        &BlockInfo::default(),
                        &transaction,
                        platform_version,
                        false,
                        None,
                    )
                    .expect("expected to process state transition");

                assert_matches!(
                    processing_result.execution_results().as_slice(),
                    [StateTransitionExecutionResult::PaidConsensusError(
                        ConsensusError::StateError(
                            StateError::IdentityInTokenConfigurationNotFoundError(_)
                        ),
                        _
                    )]
                );

                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .expect("expected to commit transaction");

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id,
                        identity_id.to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, None);
            }

            #[test]
            fn test_data_contract_creation_with_single_token_setting_pre_programmed_distribution_to_identity_that_does_not_exist(
            ) {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, 958, dash_to_credits!(1.0));

                let mut data_contract = json_document_to_contract_with_ids(
                    "tests/supporting_files/contract/basic-token/basic-token.json",
                    None,
                    None,
                    false, //no need to validate the data contracts in tests for drive
                    platform_version,
                )
                .expect("expected to get json based contract");

                let (identity_2, _, _) = setup_identity(&mut platform, 5456, dash_to_credits!(0.1));

                let (identity_3, _, _) = setup_identity(&mut platform, 123, dash_to_credits!(0.1));

                let (identity_4, _, _) = setup_identity(&mut platform, 548, dash_to_credits!(0.1));

                let identity_id = identity.id();

                let base_supply_start_amount = 0;

                let token_config = data_contract
                    .tokens_mut()
                    .expect("expected tokens")
                    .get_mut(&0)
                    .expect("expected first token");
                token_config.set_base_supply(base_supply_start_amount);

                // Create a new BTreeMap to store distributions
                let mut distributions: BTreeMap<
                    TimestampMillis,
                    BTreeMap<Identifier, TokenAmount>,
                > = BTreeMap::new();

                // Create distributions for different timestamps
                distributions.insert(
                    1700000000000, // Example timestamp (milliseconds)
                    BTreeMap::from([
                        (identity.id(), 10),  // Identity 1 gets 10 tokens
                        (identity_2.id(), 5), // Identity 2 gets 5 tokens
                    ]),
                );

                distributions.insert(
                    1700005000000, // Another timestamp
                    BTreeMap::from([
                        (identity_3.id(), 15),          // Identity 3 gets 15 tokens
                        (identity_4.id(), 20),          // Identity 4 gets 20 tokens
                        (Identifier::new([6; 32]), 25), // Identifier does not exist
                    ]),
                );

                token_config
                    .distribution_rules_mut()
                    .set_pre_programmed_distribution(Some(TokenPreProgrammedDistribution::V0(
                        TokenPreProgrammedDistributionV0 {
                            distributions: distributions.clone(),
                        },
                    )));

                let data_contract_id = DataContract::generate_data_contract_id_v0(identity_id, 1);

                let token_id = calculate_token_id(data_contract_id.as_bytes(), 0);

                let data_contract_create_transition =
                    DataContractCreateTransition::new_from_data_contract(
                        data_contract,
                        1,
                        &identity.into_partial_identity_info(),
                        key.id(),
                        &signer,
                        platform_version,
                        None,
                    )
                    .expect("expect to create documents batch transition");

                let data_contract_create_serialized_transition = data_contract_create_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &[data_contract_create_serialized_transition.clone()],
                        &platform_state,
                        &BlockInfo::default(),
                        &transaction,
                        platform_version,
                        false,
                        None,
                    )
                    .expect("expected to process state transition");

                assert_matches!(
                    processing_result.execution_results().as_slice(),
                    [StateTransitionExecutionResult::PaidConsensusError(
                        ConsensusError::StateError(
                            StateError::IdentityInTokenConfigurationNotFoundError(_)
                        ),
                        _
                    )]
                );

                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .expect("expected to commit transaction");

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id,
                        identity_id.to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, None);
            }

            #[test]
            fn test_data_contract_creation_with_single_token_setting_burn_of_external_token_not_allowed(
            ) {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let platform_state = platform.state.load();

                let (identity, contract_signer, contract_key) =
                    setup_identity(&mut platform, 958, dash_to_credits!(1.0));

                let (token_contract_owner_id, _, _) =
                    setup_identity(&mut platform, 11, dash_to_credits!(0.1));

                let (token_contract, _) = create_token_contract_with_owner_identity(
                    &mut platform,
                    token_contract_owner_id.id(),
                    None::<fn(&mut TokenConfiguration)>,
                    None,
                    None,
                    platform_version,
                );

                let token_contract_id = token_contract.id();

                let mut data_contract = json_document_to_contract_with_ids(
                    "tests/supporting_files/contract/crypto-card-game/crypto-card-game-use-external-currency.json",
                    None,
                    None,
                    false, //no need to validate the data contracts in tests for drive
                    platform_version,
                )
                    .expect("expected to get json based contract");

                {
                    let document_type = data_contract
                        .document_types_mut()
                        .get_mut("card")
                        .expect("expected a document type with name card");
                    document_type.set_document_creation_token_cost(Some(DocumentActionTokenCost {
                        contract_id: Some(token_contract_id),
                        token_contract_position: 0,
                        token_amount: 5,
                        effect: DocumentActionTokenEffect::BurnToken,
                        gas_fees_paid_by: GasFeesPaidBy::DocumentOwner,
                    }));
                    let gas_fees_paid_by_int: u8 = GasFeesPaidBy::DocumentOwner.into();
                    let schema = document_type.schema_mut();
                    let token_cost = schema
                        .get_mut("tokenCost")
                        .expect("expected to get token cost")
                        .expect("expected token cost to be set");
                    let creation_token_cost = token_cost
                        .get_mut("create")
                        .expect("expected to get creation token cost")
                        .expect("expected creation token cost to be set");
                    creation_token_cost
                        .set_value("contractId", token_contract_id.into())
                        .expect("expected to set token contract id");
                    creation_token_cost
                        .set_value("tokenPosition", 0.into())
                        .expect("expected to set token position");
                    creation_token_cost
                        .set_value("amount", 5.into())
                        .expect("expected to set token amount");
                    creation_token_cost
                        .set_value(
                            "effect",
                            Value::U8(DocumentActionTokenEffect::BurnToken.into()),
                        )
                        .expect("expected to set token pay effect");
                    creation_token_cost
                        .set_value("gasFeesPaidBy", gas_fees_paid_by_int.into())
                        .expect("expected to set token amount");
                }

                let data_contract_create_transition =
                    DataContractCreateTransition::new_from_data_contract(
                        data_contract,
                        1,
                        &identity.into_partial_identity_info(),
                        contract_key.id(),
                        &contract_signer,
                        platform_version,
                        None,
                    )
                    .expect("expect to create data contract create batch transition");

                let data_contract_create_serialized_transition = data_contract_create_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &[data_contract_create_serialized_transition.clone()],
                        &platform_state,
                        &BlockInfo::default(),
                        &transaction,
                        platform_version,
                        false,
                        None,
                    )
                    .expect("expected to process state transition");

                assert_matches!(
                    processing_result.execution_results().as_slice(),
                    [StateTransitionExecutionResult::PaidConsensusError(
                        ConsensusError::BasicError(
                            BasicError::TokenPaymentByBurningOnlyAllowedOnInternalTokenError(_)
                        ),
                        _
                    )]
                );

                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .expect("expected to commit transaction");
            }

            #[test]
            fn test_data_contract_creation_with_single_token_with_invalid_perpetual_distribution_should_cause_error(
            ) {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, 958, dash_to_credits!(1.0));

                let mut data_contract = json_document_to_contract_with_ids(
                    "tests/supporting_files/contract/basic-token/basic-token.json",
                    None,
                    None,
                    false, //no need to validate the data contracts in tests for drive
                    platform_version,
                )
                .expect("expected to get json based contract");

                {
                    let token_config = data_contract
                        .tokens_mut()
                        .expect("expected tokens")
                        .get_mut(&0)
                        .expect("expected first token");
                    token_config
                        .distribution_rules_mut()
                        .set_perpetual_distribution(Some(TokenPerpetualDistribution::V0(
                            TokenPerpetualDistributionV0 {
                                distribution_type: RewardDistributionType::BlockBasedDistribution {
                                    interval: 10,
                                    function: DistributionFunction::Exponential {
                                        a: 0,
                                        d: 0,
                                        m: 0,
                                        n: 0,
                                        o: 0,
                                        start_moment: None,
                                        b: 0,
                                        min_value: None,
                                        max_value: None,
                                    },
                                },
                                // we give to identity 2
                                distribution_recipient: TokenDistributionRecipient::Identity(
                                    identity.id(),
                                ),
                            },
                        )));
                }

                let identity_id = identity.id();

                let data_contract_id = DataContract::generate_data_contract_id_v0(identity_id, 1);

                let data_contract_create_transition =
                    DataContractCreateTransition::new_from_data_contract(
                        data_contract,
                        1,
                        &identity.into_partial_identity_info(),
                        key.id(),
                        &signer,
                        platform_version,
                        None,
                    )
                    .expect("expect to create documents batch transition");

                let token_id = calculate_token_id(data_contract_id.as_bytes(), 0);

                let data_contract_create_serialized_transition = data_contract_create_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &[data_contract_create_serialized_transition.clone()],
                        &platform_state,
                        &BlockInfo::default(),
                        &transaction,
                        platform_version,
                        false,
                        None,
                    )
                    .expect("expected to process state transition");
                assert_matches!(
                    processing_result.execution_results().as_slice(),
                    [StateTransitionExecutionResult::UnpaidConsensusError(
                        ConsensusError::BasicError(
                            BasicError::InvalidTokenDistributionFunctionDivideByZeroError(_)
                        ),
                    )]
                );

                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .expect("expected to commit transaction");

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id,
                        identity_id.to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, None);
            }

            #[test]
            fn test_data_contract_creation_with_single_token_with_random_perpetual_distribution_should_cause_error(
            ) {
                let platform_version = PlatformVersion::latest();
                let mut platform = TestPlatformBuilder::new()
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let platform_state = platform.state.load();

                let (identity, signer, key) =
                    setup_identity(&mut platform, 958, dash_to_credits!(1.0));

                let mut data_contract = json_document_to_contract_with_ids(
                    "tests/supporting_files/contract/basic-token/basic-token.json",
                    None,
                    None,
                    false, //no need to validate the data contracts in tests for drive
                    platform_version,
                )
                .expect("expected to get json based contract");

                {
                    let token_config = data_contract
                        .tokens_mut()
                        .expect("expected tokens")
                        .get_mut(&0)
                        .expect("expected first token");
                    token_config
                        .distribution_rules_mut()
                        .set_perpetual_distribution(Some(TokenPerpetualDistribution::V0(
                            TokenPerpetualDistributionV0 {
                                distribution_type: RewardDistributionType::BlockBasedDistribution {
                                    interval: 10,
                                    function: DistributionFunction::Random { min: 0, max: 10 },
                                },
                                // we give to identity 2
                                distribution_recipient: TokenDistributionRecipient::Identity(
                                    identity.id(),
                                ),
                            },
                        )));
                }

                let identity_id = identity.id();

                let data_contract_id = DataContract::generate_data_contract_id_v0(identity_id, 1);

                let data_contract_create_transition =
                    DataContractCreateTransition::new_from_data_contract(
                        data_contract,
                        1,
                        &identity.into_partial_identity_info(),
                        key.id(),
                        &signer,
                        platform_version,
                        None,
                    )
                    .expect("expect to create documents batch transition");

                let token_id = calculate_token_id(data_contract_id.as_bytes(), 0);

                let data_contract_create_serialized_transition = data_contract_create_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition");

                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &[data_contract_create_serialized_transition.clone()],
                        &platform_state,
                        &BlockInfo::default(),
                        &transaction,
                        platform_version,
                        false,
                        None,
                    )
                    .expect("expected to process state transition");
                assert_matches!(
                    processing_result.execution_results().as_slice(),
                    [StateTransitionExecutionResult::UnpaidConsensusError(
                        ConsensusError::BasicError(BasicError::UnsupportedFeatureError(_)),
                    )]
                );

                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .expect("expected to commit transaction");

                let token_balance = platform
                    .drive
                    .fetch_identity_token_balance(
                        token_id,
                        identity_id.to_buffer(),
                        None,
                        platform_version,
                    )
                    .expect("expected to fetch token balance");
                assert_eq!(token_balance, None);
            }
        }
    }

    mod group_errors {
        use super::*;
        #[test]
        fn test_data_contract_creation_with_non_contiguous_groups_should_error() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));

            let (identity_2, _, _) = setup_identity(&mut platform, 234, dash_to_credits!(0.1));

            let (identity_3, _, _) = setup_identity(&mut platform, 45, dash_to_credits!(0.1));

            let mut data_contract = json_document_to_contract_with_ids(
                "tests/supporting_files/contract/basic-token/basic-token.json",
                None,
                None,
                false, //no need to validate the data contracts in tests for drive
                platform_version,
            )
            .expect("expected to get json based contract");

            let identity_id = identity.id();

            let base_supply_start_amount = 0;

            {
                let groups = data_contract.groups_mut().expect("expected tokens");
                groups.insert(
                    0,
                    Group::V0(GroupV0 {
                        members: [(identity.id(), 1), (identity_2.id(), 1)].into(),
                        required_power: 2,
                    }),
                );
                groups.insert(
                    2,
                    Group::V0(GroupV0 {
                        members: [
                            (identity.id(), 1),
                            (identity_3.id(), 1),
                            (identity_2.id(), 2),
                        ]
                        .into(),
                        required_power: 2,
                    }),
                );
                let token_config = data_contract
                    .tokens_mut()
                    .expect("expected tokens")
                    .get_mut(&0)
                    .expect("expected first token");
                token_config.set_main_control_group(Some(2));
                token_config.set_base_supply(base_supply_start_amount);
                token_config.set_manual_minting_rules(ChangeControlRules::V0(
                    ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::Group(0),
                        // We have no group at position 1, we should get an error
                        admin_action_takers: AuthorizedActionTakers::MainGroup,
                        changing_authorized_action_takers_to_no_one_allowed: false,
                        changing_admin_action_takers_to_no_one_allowed: false,
                        self_changing_admin_action_takers_allowed: false,
                    },
                ));
            }

            let data_contract_id = DataContract::generate_data_contract_id_v0(identity_id, 1);

            let token_id = calculate_token_id(data_contract_id.as_bytes(), 0);

            let data_contract_create_transition =
                DataContractCreateTransition::new_from_data_contract(
                    data_contract,
                    1,
                    &identity.into_partial_identity_info(),
                    key.id(),
                    &signer,
                    platform_version,
                    None,
                )
                .expect("expect to create documents batch transition");

            let data_contract_create_serialized_transition = data_contract_create_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[data_contract_create_serialized_transition.clone()],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(
                        BasicError::NonContiguousContractGroupPositionsError(_)
                    ),
                )]
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            let token_balance = platform
                .drive
                .fetch_identity_token_balance(
                    token_id,
                    identity_id.to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, None);
        }

        #[test]
        fn test_data_contract_creation_with_group_with_member_with_zero_power_should_error() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));

            let (identity_2, _, _) = setup_identity(&mut platform, 234, dash_to_credits!(0.1));

            let (identity_3, _, _) = setup_identity(&mut platform, 45, dash_to_credits!(0.1));

            let mut data_contract = json_document_to_contract_with_ids(
                "tests/supporting_files/contract/basic-token/basic-token.json",
                None,
                None,
                false, //no need to validate the data contracts in tests for drive
                platform_version,
            )
            .expect("expected to get json based contract");

            let identity_id = identity.id();

            let base_supply_start_amount = 0;

            {
                let groups = data_contract.groups_mut().expect("expected tokens");
                groups.insert(
                    0,
                    Group::V0(GroupV0 {
                        members: [(identity.id(), 1), (identity_2.id(), 1)].into(),
                        required_power: 2,
                    }),
                );
                groups.insert(
                    1,
                    Group::V0(GroupV0 {
                        members: [
                            (identity.id(), 1),
                            (identity_3.id(), 0), //error
                            (identity_2.id(), 2),
                        ]
                        .into(),
                        required_power: 2,
                    }),
                );
                let token_config = data_contract
                    .tokens_mut()
                    .expect("expected tokens")
                    .get_mut(&0)
                    .expect("expected first token");
                token_config.set_main_control_group(Some(1));
                token_config.set_base_supply(base_supply_start_amount);
                token_config.set_manual_minting_rules(ChangeControlRules::V0(
                    ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::Group(0),
                        admin_action_takers: AuthorizedActionTakers::MainGroup,
                        changing_authorized_action_takers_to_no_one_allowed: false,
                        changing_admin_action_takers_to_no_one_allowed: false,
                        self_changing_admin_action_takers_allowed: false,
                    },
                ));
            }

            let data_contract_id = DataContract::generate_data_contract_id_v0(identity_id, 1);

            let token_id = calculate_token_id(data_contract_id.as_bytes(), 0);

            let data_contract_create_transition =
                DataContractCreateTransition::new_from_data_contract(
                    data_contract,
                    1,
                    &identity.into_partial_identity_info(),
                    key.id(),
                    &signer,
                    platform_version,
                    None,
                )
                .expect("expect to create documents batch transition");

            let data_contract_create_serialized_transition = data_contract_create_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[data_contract_create_serialized_transition.clone()],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::GroupMemberHasPowerOfZeroError(_)),
                )]
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            let token_balance = platform
                .drive
                .fetch_identity_token_balance(
                    token_id,
                    identity_id.to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, None);
        }

        #[test]
        fn test_data_contract_creation_with_group_with_member_with_too_big_power_should_error() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));

            let (identity_2, _, _) = setup_identity(&mut platform, 234, dash_to_credits!(0.1));

            let (identity_3, _, _) = setup_identity(&mut platform, 45, dash_to_credits!(0.1));

            let mut data_contract = json_document_to_contract_with_ids(
                "tests/supporting_files/contract/basic-token/basic-token.json",
                None,
                None,
                false, //no need to validate the data contracts in tests for drive
                platform_version,
            )
            .expect("expected to get json based contract");

            let identity_id = identity.id();

            let base_supply_start_amount = 0;

            {
                let groups = data_contract.groups_mut().expect("expected tokens");
                groups.insert(
                    0,
                    Group::V0(GroupV0 {
                        members: [(identity.id(), 1), (identity_2.id(), 1)].into(),
                        required_power: 2,
                    }),
                );
                groups.insert(
                    1,
                    Group::V0(GroupV0 {
                        members: [
                            (identity.id(), 50000),
                            (identity_3.id(), 100000), //error
                            (identity_2.id(), 50000),
                        ]
                        .into(),
                        required_power: 100000,
                    }),
                );
                let token_config = data_contract
                    .tokens_mut()
                    .expect("expected tokens")
                    .get_mut(&0)
                    .expect("expected first token");
                token_config.set_main_control_group(Some(1));
                token_config.set_base_supply(base_supply_start_amount);
                token_config.set_manual_minting_rules(ChangeControlRules::V0(
                    ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::Group(0),
                        admin_action_takers: AuthorizedActionTakers::MainGroup,
                        changing_authorized_action_takers_to_no_one_allowed: false,
                        changing_admin_action_takers_to_no_one_allowed: false,
                        self_changing_admin_action_takers_allowed: false,
                    },
                ));
            }

            let data_contract_id = DataContract::generate_data_contract_id_v0(identity_id, 1);

            let token_id = calculate_token_id(data_contract_id.as_bytes(), 0);

            let data_contract_create_transition =
                DataContractCreateTransition::new_from_data_contract(
                    data_contract,
                    1,
                    &identity.into_partial_identity_info(),
                    key.id(),
                    &signer,
                    platform_version,
                    None,
                )
                .expect("expect to create documents batch transition");

            let data_contract_create_serialized_transition = data_contract_create_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[data_contract_create_serialized_transition.clone()],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::GroupMemberHasPowerOverLimitError(_)),
                )]
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            let token_balance = platform
                .drive
                .fetch_identity_token_balance(
                    token_id,
                    identity_id.to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, None);
        }

        #[test]
        fn test_data_contract_creation_with_group_with_member_with_power_over_required_should_error(
        ) {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));

            let (identity_2, _, _) = setup_identity(&mut platform, 234, dash_to_credits!(0.1));

            let (identity_3, _, _) = setup_identity(&mut platform, 45, dash_to_credits!(0.1));

            let mut data_contract = json_document_to_contract_with_ids(
                "tests/supporting_files/contract/basic-token/basic-token.json",
                None,
                None,
                false, //no need to validate the data contracts in tests for drive
                platform_version,
            )
            .expect("expected to get json based contract");

            let identity_id = identity.id();

            let base_supply_start_amount = 0;

            {
                let groups = data_contract.groups_mut().expect("expected tokens");
                groups.insert(
                    0,
                    Group::V0(GroupV0 {
                        members: [(identity.id(), 1), (identity_2.id(), 1)].into(),
                        required_power: 2,
                    }),
                );
                groups.insert(
                    1,
                    Group::V0(GroupV0 {
                        members: [
                            (identity.id(), 3),
                            (identity_3.id(), 6), //error
                            (identity_2.id(), 3),
                        ]
                        .into(),
                        required_power: 5,
                    }),
                );
                let token_config = data_contract
                    .tokens_mut()
                    .expect("expected tokens")
                    .get_mut(&0)
                    .expect("expected first token");
                token_config.set_main_control_group(Some(1));
                token_config.set_base_supply(base_supply_start_amount);
                token_config.set_manual_minting_rules(ChangeControlRules::V0(
                    ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::Group(0),
                        admin_action_takers: AuthorizedActionTakers::MainGroup,
                        changing_authorized_action_takers_to_no_one_allowed: false,
                        changing_admin_action_takers_to_no_one_allowed: false,
                        self_changing_admin_action_takers_allowed: false,
                    },
                ));
            }

            let data_contract_id = DataContract::generate_data_contract_id_v0(identity_id, 1);

            let token_id = calculate_token_id(data_contract_id.as_bytes(), 0);

            let data_contract_create_transition =
                DataContractCreateTransition::new_from_data_contract(
                    data_contract,
                    1,
                    &identity.into_partial_identity_info(),
                    key.id(),
                    &signer,
                    platform_version,
                    None,
                )
                .expect("expect to create documents batch transition");

            let data_contract_create_serialized_transition = data_contract_create_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[data_contract_create_serialized_transition.clone()],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::GroupMemberHasPowerOverLimitError(_)),
                )]
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            let token_balance = platform
                .drive
                .fetch_identity_token_balance(
                    token_id,
                    identity_id.to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, None);
        }

        #[test]
        fn test_dcc_group_with_member_power_not_reaching_threshold() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));

            let (identity_2, _, _) = setup_identity(&mut platform, 234, dash_to_credits!(0.1));

            let (identity_3, _, _) = setup_identity(&mut platform, 45, dash_to_credits!(0.1));

            let mut data_contract = json_document_to_contract_with_ids(
                "tests/supporting_files/contract/basic-token/basic-token.json",
                None,
                None,
                false, //no need to validate the data contracts in tests for drive
                platform_version,
            )
            .expect("expected to get json based contract");

            let identity_id = identity.id();

            let base_supply_start_amount = 0;

            {
                let groups = data_contract.groups_mut().expect("expected tokens");
                groups.insert(
                    0,
                    Group::V0(GroupV0 {
                        members: [(identity.id(), 1), (identity_2.id(), 1)].into(),
                        required_power: 2,
                    }),
                );
                groups.insert(
                    1,
                    Group::V0(GroupV0 {
                        members: [
                            (identity.id(), 1),
                            (identity_3.id(), 1),
                            (identity_2.id(), 1),
                        ]
                        .into(),
                        required_power: 5, // 1 + 1 + 1 < 5 so we should error
                    }),
                );
                let token_config = data_contract
                    .tokens_mut()
                    .expect("expected tokens")
                    .get_mut(&0)
                    .expect("expected first token");
                token_config.set_main_control_group(Some(1));
                token_config.set_base_supply(base_supply_start_amount);
                token_config.set_manual_minting_rules(ChangeControlRules::V0(
                    ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::Group(0),
                        admin_action_takers: AuthorizedActionTakers::MainGroup,
                        changing_authorized_action_takers_to_no_one_allowed: false,
                        changing_admin_action_takers_to_no_one_allowed: false,
                        self_changing_admin_action_takers_allowed: false,
                    },
                ));
            }

            let data_contract_id = DataContract::generate_data_contract_id_v0(identity_id, 1);

            let token_id = calculate_token_id(data_contract_id.as_bytes(), 0);

            let data_contract_create_transition =
                DataContractCreateTransition::new_from_data_contract(
                    data_contract,
                    1,
                    &identity.into_partial_identity_info(),
                    key.id(),
                    &signer,
                    platform_version,
                    None,
                )
                .expect("expect to create documents batch transition");

            let data_contract_create_serialized_transition = data_contract_create_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[data_contract_create_serialized_transition.clone()],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::GroupTotalPowerLessThanRequiredError(_)),
                )]
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            let token_balance = platform
                .drive
                .fetch_identity_token_balance(
                    token_id,
                    identity_id.to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, None);
        }

        #[test]
        fn test_dcc_group_with_non_unilateral_member_power_not_reaching_threshold() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));

            let (identity_2, _, _) = setup_identity(&mut platform, 234, dash_to_credits!(0.1));

            let (identity_3, _, _) = setup_identity(&mut platform, 45, dash_to_credits!(0.1));

            let mut data_contract = json_document_to_contract_with_ids(
                "tests/supporting_files/contract/basic-token/basic-token.json",
                None,
                None,
                false, //no need to validate the data contracts in tests for drive
                platform_version,
            )
            .expect("expected to get json based contract");

            let identity_id = identity.id();

            let base_supply_start_amount = 0;

            {
                let groups = data_contract.groups_mut().expect("expected tokens");
                groups.insert(
                    0,
                    Group::V0(GroupV0 {
                        members: [(identity.id(), 1), (identity_2.id(), 1)].into(),
                        required_power: 2,
                    }),
                );
                groups.insert(
                    1,
                    Group::V0(GroupV0 {
                        members: [
                            (identity.id(), 1),
                            (identity_3.id(), 5),
                            (identity_2.id(), 1),
                        ]
                        .into(),
                        required_power: 5, // 1 + 1 < 5 so we should error
                    }),
                );
                let token_config = data_contract
                    .tokens_mut()
                    .expect("expected tokens")
                    .get_mut(&0)
                    .expect("expected first token");
                token_config.set_main_control_group(Some(1));
                token_config.set_base_supply(base_supply_start_amount);
                token_config.set_manual_minting_rules(ChangeControlRules::V0(
                    ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::Group(0),
                        admin_action_takers: AuthorizedActionTakers::MainGroup,
                        changing_authorized_action_takers_to_no_one_allowed: false,
                        changing_admin_action_takers_to_no_one_allowed: false,
                        self_changing_admin_action_takers_allowed: false,
                    },
                ));
            }

            let data_contract_id = DataContract::generate_data_contract_id_v0(identity_id, 1);

            let token_id = calculate_token_id(data_contract_id.as_bytes(), 0);

            let data_contract_create_transition =
                DataContractCreateTransition::new_from_data_contract(
                    data_contract,
                    1,
                    &identity.into_partial_identity_info(),
                    key.id(),
                    &signer,
                    platform_version,
                    None,
                )
                .expect("expect to create documents batch transition");

            let data_contract_create_serialized_transition = data_contract_create_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[data_contract_create_serialized_transition.clone()],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(
                        BasicError::GroupNonUnilateralMemberPowerHasLessThanRequiredPowerError(_)
                    ),
                )]
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            let token_balance = platform
                .drive
                .fetch_identity_token_balance(
                    token_id,
                    identity_id.to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, None);
        }
    }

    mod keywords {
        use super::*;
        use dpp::{
            data_contract::conversion::value::v0::DataContractValueConversionMethodsV0,
            data_contracts::SystemDataContract, document::DocumentV0Getters,
            platform_value::string_encoding::Encoding,
            system_data_contracts::load_system_data_contract,
        };
        use drive::{
            drive::document::query::QueryDocumentsOutcomeV0Methods, query::DriveDocumentQuery,
        };

        #[test]
        fn test_data_contract_creation_fails_with_more_than_fifty_keywords() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let platform_state = platform.state.load();

            // Create a test identity and keys
            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));

            // Load the base contract JSON and convert it to `DataContract`
            let data_contract = json_document_to_contract_with_ids(
                "tests/supporting_files/contract/keyword_test/keyword_base_contract.json",
                None,
                None,
                false,
                platform_version,
            )
            .expect("expected to load contract");

            // Convert the contract back to Value so we can mutate its fields
            let mut contract_value = data_contract
                .to_value(PlatformVersion::latest())
                .expect("to_value failed");

            // Insert 21 keywords to exceed the max limit
            let mut excessive_keywords: Vec<Value> = vec![];
            for i in 0..51 {
                excessive_keywords.push(Value::Text(format!("keyword{}", i)));
            }
            contract_value["keywords"] = Value::Array(excessive_keywords);

            // Build a new DataContract from the mutated Value
            let data_contract_with_excessive_keywords =
                DataContract::from_value(contract_value, true, platform_version)
                    .expect("failed to create DataContract from Value");

            // Create the DataContractCreateTransition
            let data_contract_create_transition =
                DataContractCreateTransition::new_from_data_contract(
                    data_contract_with_excessive_keywords,
                    1,
                    &identity.into_partial_identity_info(),
                    key.id(),
                    &signer,
                    platform_version,
                    None,
                )
                .expect("expect to create data contract transition");

            // Serialize the transition
            let data_contract_create_serialized_transition = data_contract_create_transition
                .serialize_to_bytes()
                .expect("expected a serialized data contract transition");

            let transaction = platform.drive.grove.start_transaction();

            // Process the state transition
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[data_contract_create_serialized_transition],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // We expect a failure due to the JSON schema rejecting >20 keywords
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::TooManyKeywordsError(_))
                )]
            );
        }

        #[test]
        fn test_data_contract_creation_fails_with_duplicate_keywords() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let platform_state = platform.state.load();

            // Create a test identity and keys
            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));

            // Load the base contract JSON and convert it to `DataContract`
            let data_contract = json_document_to_contract_with_ids(
                "tests/supporting_files/contract/keyword_test/keyword_base_contract.json",
                None,
                None,
                false,
                platform_version,
            )
            .expect("expected to load contract");

            // Convert to Value to mutate fields
            let mut contract_value = data_contract
                .to_value(PlatformVersion::latest())
                .expect("to_value failed");

            // Insert some duplicates
            let duplicated_keywords = vec!["keyword1", "keyword2", "keyword2"];
            contract_value["keywords"] = Value::Array(
                duplicated_keywords
                    .into_iter()
                    .map(|str| Value::Text(str.to_string()))
                    .collect(),
            );

            // Build a new DataContract from the mutated Value
            let data_contract_with_duplicates =
                DataContract::from_value(contract_value, true, platform_version)
                    .expect("failed to create DataContract from Value");

            // Create the DataContractCreateTransition
            let data_contract_create_transition =
                DataContractCreateTransition::new_from_data_contract(
                    data_contract_with_duplicates,
                    1,
                    &identity.into_partial_identity_info(),
                    key.id(),
                    &signer,
                    platform_version,
                    None,
                )
                .expect("expect to create data contract transition");

            // Serialize the transition
            let data_contract_create_serialized_transition = data_contract_create_transition
                .serialize_to_bytes()
                .expect("expected a serialized data contract transition");

            let transaction = platform.drive.grove.start_transaction();

            // Process the state transition
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[data_contract_create_serialized_transition],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Expect failure
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::DuplicateKeywordsError(_))
                )]
            );
        }

        #[test]
        fn test_data_contract_creation_fails_with_keyword_too_short() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let platform_state = platform.state.load();

            // Create identity
            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));

            // Load the base contract JSON and convert it to `DataContract`
            let data_contract = json_document_to_contract_with_ids(
                "tests/supporting_files/contract/keyword_test/keyword_base_contract.json",
                None,
                None,
                false,
                platform_version,
            )
            .expect("expected to load contract");

            // Convert to Value for mutation
            let mut contract_value = data_contract
                .to_value(PlatformVersion::latest())
                .expect("to_value failed");

            // Insert a keyword with length < 3
            contract_value["keywords"] = Value::Array(vec![Value::Text("hi".to_string())]);

            // Build a new DataContract
            let data_contract_invalid =
                DataContract::from_value(contract_value, true, platform_version)
                    .expect("failed to create DataContract");

            // Create DataContractCreateTransition
            let data_contract_create_transition =
                DataContractCreateTransition::new_from_data_contract(
                    data_contract_invalid,
                    1,
                    &identity.into_partial_identity_info(),
                    key.id(),
                    &signer,
                    platform_version,
                    None,
                )
                .expect("expect to create transition");

            // Process
            let data_contract_create_serialized_transition = data_contract_create_transition
                .serialize_to_bytes()
                .expect("expected to serialize");
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[data_contract_create_serialized_transition],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Assert that we get the correct error
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::InvalidKeywordLengthError(_))
                )]
            );
        }

        #[test]
        fn test_data_contract_creation_fails_with_keyword_too_long() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let platform_state = platform.state.load();
            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));

            let data_contract = json_document_to_contract_with_ids(
                "tests/supporting_files/contract/keyword_test/keyword_base_contract.json",
                None,
                None,
                false,
                platform_version,
            )
            .expect("expected to load contract");

            let mut contract_value = data_contract
                .to_value(platform_version)
                .expect("to_value failed");

            // Create a 51-char keyword
            let too_long_keyword = "x".repeat(51);
            contract_value["keywords"] = Value::Array(vec![Value::Text(too_long_keyword)]);

            let data_contract_invalid =
                DataContract::from_value(contract_value, true, platform_version)
                    .expect("failed to create DataContract");

            let data_contract_create_transition =
                DataContractCreateTransition::new_from_data_contract(
                    data_contract_invalid,
                    1,
                    &identity.into_partial_identity_info(),
                    key.id(),
                    &signer,
                    platform_version,
                    None,
                )
                .expect("expect to create transition");

            let data_contract_create_serialized_transition = data_contract_create_transition
                .serialize_to_bytes()
                .expect("expected to serialize");

            let transaction = platform.drive.grove.start_transaction();
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[data_contract_create_serialized_transition],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::InvalidKeywordLengthError(_))
                )]
            );
        }

        #[test]
        fn test_data_contract_creation_succeeds_with_valid_keywords() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let platform_state = platform.state.load();

            // Create a test identity and keys
            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));

            // Load the base contract JSON and convert to `DataContract`
            let data_contract = json_document_to_contract_with_ids(
                "tests/supporting_files/contract/keyword_test/keyword_base_contract.json",
                None,
                None,
                false,
                platform_version,
            )
            .expect("expected to load contract");

            // Convert to Value so we can adjust fields if needed
            let mut contract_value = data_contract
                .to_value(PlatformVersion::latest())
                .expect("to_value failed");

            // Insert a valid set of keywords: all distinct, fewer than 20
            let valid_keywords = vec!["key1", "key2", "key3"];
            contract_value["keywords"] = Value::Array(
                valid_keywords
                    .into_iter()
                    .map(|str| Value::Text(str.to_string()))
                    .collect(),
            );

            // Build a new DataContract from the mutated Value
            let data_contract_valid =
                DataContract::from_value(contract_value, true, platform_version)
                    .expect("failed to create DataContract from Value");

            // Create the DataContractCreateTransition
            let data_contract_create_transition =
                DataContractCreateTransition::new_from_data_contract(
                    data_contract_valid,
                    1,
                    &identity.into_partial_identity_info(),
                    key.id(),
                    &signer,
                    platform_version,
                    None,
                )
                .expect("expect to create data contract transition");

            // Serialize the transition
            let data_contract_create_serialized_transition = data_contract_create_transition
                .serialize_to_bytes()
                .expect("expected a serialized data contract transition");

            let transaction = platform.drive.grove.start_transaction();

            // Process the state transition
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[data_contract_create_serialized_transition],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // This time we expect success
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );

            // Commit the transaction since it's valid
            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            // Get the data contract ID from the transition
            // Is there a simpler way to get the ID?
            let unique_identifiers = data_contract_create_transition.unique_identifiers();
            let unique_identifier = unique_identifiers
                .first()
                .expect("expected at least one unique identifier");
            let unique_identifier_str = unique_identifier.as_str();
            let data_contract_id_str = unique_identifier_str
                .split('-')
                .last()
                .expect("expected to extract data contract id from unique identifier");
            let data_contract_id = Identifier::from_string(data_contract_id_str, Encoding::Base58)
                .expect("failed to create Identifier from string");

            // Fetch the contract from the platform
            let contract = platform
                .drive
                .fetch_contract(data_contract_id.into(), None, None, None, platform_version)
                .value
                .expect("expected to get contract")
                .expect("expected to find the contract");

            // Check the keywords in the contract
            let keywords = contract.contract.keywords();
            assert_eq!(keywords.len(), 3);
            assert_eq!(keywords[0], "key1");
            assert_eq!(keywords[1], "key2");
            assert_eq!(keywords[2], "key3");

            // Now check the Search Contract has the keyword documents
            let search_contract = load_system_data_contract(
                SystemDataContract::KeywordSearch,
                PlatformVersion::latest(),
            )
            .expect("expected to load search contract");
            let document_type = search_contract
                .document_type_for_name("contractKeywords")
                .expect("expected to get document type");

            let drive_query =
                DriveDocumentQuery::all_items_query(&search_contract, document_type, None);

            let documents_result = platform
                .drive
                .query_documents(drive_query, None, false, None, None)
                .expect("expected to query documents");

            let documents = documents_result.documents();

            assert_eq!(documents.len(), 3);

            let mut valid_keywords_for_verification = vec!["key1", "key2", "key3"];
            for document in documents {
                let keyword = document
                    .get("keyword")
                    .expect("expected to get keyword")
                    .as_str()
                    .expect("expected to get string");

                assert!(valid_keywords_for_verification.contains(&keyword));
                assert_eq!(
                    document
                        .get("contractId")
                        .expect("expected to get data contract id")
                        .clone()
                        .into_identifier()
                        .expect("expected to get identifier")
                        .to_string(Encoding::Base58),
                    data_contract_id_str
                );
                valid_keywords_for_verification.retain(|&x| x != keyword);
            }
        }
    }

    mod descriptions {
        use dpp::{
            data_contract::conversion::value::v0::DataContractValueConversionMethodsV0,
            data_contracts::SystemDataContract, document::DocumentV0Getters,
            platform_value::string_encoding::Encoding,
            system_data_contracts::load_system_data_contract,
        };
        use drive::{
            drive::document::query::QueryDocumentsOutcomeV0Methods, query::DriveDocumentQuery,
        };

        use super::*;

        /// Returns a `DataContract` value that already contains at least one keyword
        fn base_contract_value_with_keyword(platform_version: &PlatformVersion) -> Value {
            let data_contract = json_document_to_contract_with_ids(
                // Re‑use the same fixture you already have; it doesn’t need
                // to contain a description field – we mutate it below.
                "tests/supporting_files/contract/keyword_test/keyword_base_contract.json",
                None,
                None,
                false,
                platform_version,
            )
            .expect("expected to load contract");

            let mut contract_value = data_contract
                .to_value(PlatformVersion::latest())
                .expect("to_value failed");

            // Ensure the `keywords` array is not empty so that Drive will attempt
            // to create the description documents.
            contract_value["keywords"] = Value::Array(vec![Value::Text("key1".to_string())]);

            contract_value
        }

        #[test]
        fn test_data_contract_creation_fails_with_description_too_short() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let platform_state = platform.state.load();
            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));

            // --- mutate the contract ---
            let mut contract_value = base_contract_value_with_keyword(platform_version);
            contract_value["description"] = Value::Text("hi".to_string()); // < 3 chars

            let data_contract_invalid =
                DataContract::from_value(contract_value, true, platform_version)
                    .expect("failed to create DataContract from Value");

            let transition = DataContractCreateTransition::new_from_data_contract(
                data_contract_invalid,
                1,
                &identity.into_partial_identity_info(),
                key.id(),
                &signer,
                platform_version,
                None,
            )
            .expect("expected to create transition");

            let serialized = transition
                .serialize_to_bytes()
                .expect("expected to serialize");

            let tx = platform.drive.grove.start_transaction();
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[serialized],
                    &platform_state,
                    &BlockInfo::default(),
                    &tx,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected processing");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::InvalidDescriptionLengthError(_))
                )]
            );
        }

        #[test]
        fn test_data_contract_creation_fails_with_description_too_long() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let platform_state = platform.state.load();
            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));

            let mut contract_value = base_contract_value_with_keyword(platform_version);
            // 101 chars – valid for the contract (max 10 000) but exceeds the
            // 100‑char limit of the autogenerated **shortDescription** document.
            let too_long = "x".repeat(101);
            contract_value["description"] = Value::Text(too_long);

            let data_contract_invalid =
                DataContract::from_value(contract_value, true, platform_version)
                    .expect("failed to create DataContract");

            let transition = DataContractCreateTransition::new_from_data_contract(
                data_contract_invalid,
                1,
                &identity.into_partial_identity_info(),
                key.id(),
                &signer,
                platform_version,
                None,
            )
            .expect("expected to create transition");

            let serialized = transition
                .serialize_to_bytes()
                .expect("expected to serialize");

            let tx = platform.drive.grove.start_transaction();
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[serialized],
                    &platform_state,
                    &BlockInfo::default(),
                    &tx,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected processing");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::InvalidDescriptionLengthError(_))
                )]
            );
        }

        #[test]
        fn test_data_contract_creation_succeeds_with_valid_description() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let platform_state = platform.state.load();
            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));

            let mut contract_value = base_contract_value_with_keyword(platform_version);
            contract_value["description"] =
                Value::Text("A perfectly valid description.".to_string());

            let data_contract_valid =
                DataContract::from_value(contract_value, true, platform_version)
                    .expect("failed to create DataContract");

            let transition = DataContractCreateTransition::new_from_data_contract(
                data_contract_valid,
                1,
                &identity.into_partial_identity_info(),
                key.id(),
                &signer,
                platform_version,
                None,
            )
            .expect("expected to create transition");

            let serialized = transition
                .serialize_to_bytes()
                .expect("expected to serialize");

            let tx = platform.drive.grove.start_transaction();
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[serialized],
                    &platform_state,
                    &BlockInfo::default(),
                    &tx,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected processing");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );

            // Commit so we can query the state afterward
            platform
                .drive
                .grove
                .commit_transaction(tx)
                .unwrap()
                .expect("expected commit");

            // ---- Verify description persisted in the contract ----
            let unique_identifiers = transition.unique_identifiers();
            let unique_identifier = unique_identifiers
                .first()
                .expect("expected at least one unique identifier");
            let data_contract_id_str = unique_identifier
                .as_str()
                .split('-')
                .last()
                .expect("split contract id");
            let data_contract_id = Identifier::from_string(data_contract_id_str, Encoding::Base58)
                .expect("identifier");

            let contract = platform
                .drive
                .fetch_contract(data_contract_id.into(), None, None, None, platform_version)
                .value
                .expect("expected contract")
                .expect("contract exists");

            let desc = contract
                .contract
                .description()
                .expect("description should exist");

            assert_eq!(desc, "A perfectly valid description.");

            // Now check the Search Contract has the short and full description documents
            let search_contract = load_system_data_contract(
                SystemDataContract::KeywordSearch,
                PlatformVersion::latest(),
            )
            .expect("expected to load search contract");
            let short_description_document_type = search_contract
                .document_type_for_name("shortDescription")
                .expect("expected to get document type");
            let full_description_document_type = search_contract
                .document_type_for_name("fullDescription")
                .expect("expected to get document type");

            let drive_query_short_description = DriveDocumentQuery::all_items_query(
                &search_contract,
                short_description_document_type,
                None,
            );

            let short_description_documents_result = platform
                .drive
                .query_documents(drive_query_short_description, None, false, None, None)
                .expect("expected to query documents");

            let short_description_documents = short_description_documents_result.documents();

            assert_eq!(short_description_documents.len(), 1);
            let short_description_document = short_description_documents
                .first()
                .expect("expected to get first document");
            let short_description = short_description_document
                .get("description")
                .expect("expected to get description")
                .as_str()
                .expect("expected to get string");
            assert_eq!(short_description, "A perfectly valid description.");
            assert_eq!(
                short_description_document
                    .get("contractId")
                    .expect("expected to get data contract id")
                    .clone()
                    .into_identifier()
                    .expect("expected to get identifier")
                    .to_string(Encoding::Base58),
                data_contract_id_str
            );

            let drive_query_full_description = DriveDocumentQuery::all_items_query(
                &search_contract,
                full_description_document_type,
                None,
            );
            let full_description_documents_result = platform
                .drive
                .query_documents(drive_query_full_description, None, false, None, None)
                .expect("expected to query documents");

            let full_description_documents = full_description_documents_result.documents();

            assert_eq!(full_description_documents.len(), 1);
            let full_description_document = full_description_documents
                .first()
                .expect("expected to get first document");
            let full_description = full_description_document
                .get("description")
                .expect("expected to get description")
                .as_str()
                .expect("expected to get string");
            assert_eq!(full_description, "A perfectly valid description.");
            assert_eq!(
                full_description_document
                    .get("contractId")
                    .expect("expected to get data contract id")
                    .clone()
                    .into_identifier()
                    .expect("expected to get identifier")
                    .to_string(Encoding::Base58),
                data_contract_id_str
            );
        }
    }
}
