mod advanced_structure;
mod identity_nonce;
mod state;

use dpp::block::block_info::BlockInfo;
use dpp::identity::PartialIdentity;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
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
    StateTransitionAdvancedStructureValidationV0, StateTransitionStateValidationV0,
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
        _block_info: &BlockInfo,
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
            .basic_structure
        {
            Some(0) => self.validate_advanced_structure_v0(execution_context, platform_version),
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
                validation_mode,
                &block_info.epoch,
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
    use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use assert_matches::assert_matches;
    use dpp::block::block_info::BlockInfo;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::ConsensusError;
    use dpp::dash_to_credits;
    use dpp::data_contract::accessors::v1::DataContractV1Getters;
    use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Setters;
    use dpp::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
    use dpp::data_contract::change_control_rules::v0::ChangeControlRulesV0;
    use dpp::data_contract::change_control_rules::ChangeControlRules;
    use dpp::data_contract::group::v0::GroupV0;
    use dpp::data_contract::group::Group;
    use dpp::data_contract::DataContract;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::data_contract_create_transition::methods::DataContractCreateTransitionMethodsV0;
    use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
    use dpp::tests::json_document::json_document_to_contract_with_ids;
    use dpp::tokens::calculate_token_id;
    use platform_version::version::PlatformVersion;

    #[test]
    fn test_data_contract_creation_with_contested_unique_index() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
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
                &vec![data_contract_create_serialized_transition.clone()],
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

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

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
                &vec![data_contract_create_serialized_transition.clone()],
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

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

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
                &vec![data_contract_create_serialized_transition.clone()],
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

    #[test]
    fn test_data_contract_creation_with_single_token() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

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
                &vec![data_contract_create_serialized_transition.clone()],
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
            .fetch_identity_token_balance(token_id, identity_id.to_buffer(), None, platform_version)
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

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

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
            token_config.set_manual_minting_rules(ChangeControlRules::V0(ChangeControlRulesV0 {
                authorized_to_make_change: AuthorizedActionTakers::Group(0),
                // We have no group at position 1, we should get an error
                authorized_to_change_authorized_action_takers: AuthorizedActionTakers::MainGroup,
                changing_authorized_action_takers_to_no_one_allowed: false,
                changing_authorized_action_takers_to_contract_owner_allowed: false,
            }));
        }

        let data_contract_id = DataContract::generate_data_contract_id_v0(identity_id, 1);

        let token_id = calculate_token_id(data_contract_id.as_bytes(), 0);

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
                &vec![data_contract_create_serialized_transition.clone()],
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
            .fetch_identity_token_balance(token_id, identity_id.to_buffer(), None, platform_version)
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

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

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

        let token_id = calculate_token_id(data_contract_id.as_bytes(), 0);

        let data_contract_create_serialized_transition = data_contract_create_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![data_contract_create_serialized_transition.clone()],
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
            .fetch_identity_token_balance(token_id, identity_id.to_buffer(), None, platform_version)
            .expect("expected to fetch token balance");
        assert_eq!(token_balance, Some(base_supply_start_amount));
    }

    mod token_errors {
        use super::*;
        #[test]
        fn test_data_contract_creation_with_single_token_with_starting_balance_over_limit_should_cause_error(
        ) {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

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
                    &vec![data_contract_create_serialized_transition.clone()],
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
                    ConsensusError::BasicError(BasicError::InvalidTokenBaseSupplyError(_)),
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
        fn test_data_contract_creation_with_single_token_needing_group_that_does_not_exist() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

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
                        authorized_to_change_authorized_action_takers:
                            AuthorizedActionTakers::Group(1),
                        changing_authorized_action_takers_to_no_one_allowed: false,
                        changing_authorized_action_takers_to_contract_owner_allowed: false,
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
                    &vec![data_contract_create_serialized_transition.clone()],
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
                    ConsensusError::BasicError(BasicError::GroupPositionDoesNotExistError(_)),
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
        fn test_data_contract_creation_with_single_token_setting_main_group_that_does_not_exist() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

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
                        authorized_to_change_authorized_action_takers:
                            AuthorizedActionTakers::MainGroup,
                        changing_authorized_action_takers_to_no_one_allowed: false,
                        changing_authorized_action_takers_to_contract_owner_allowed: false,
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
                    &vec![data_contract_create_serialized_transition.clone()],
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
                    ConsensusError::BasicError(BasicError::GroupPositionDoesNotExistError(_)),
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

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

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
                        authorized_to_change_authorized_action_takers:
                            AuthorizedActionTakers::MainGroup,
                        changing_authorized_action_takers_to_no_one_allowed: false,
                        changing_authorized_action_takers_to_contract_owner_allowed: false,
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
                    &vec![data_contract_create_serialized_transition.clone()],
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
                        BasicError::NonContiguousContractGroupPositionsError(_)
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
        fn test_data_contract_creation_with_group_with_member_with_zero_power_should_error() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

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
                        authorized_to_change_authorized_action_takers:
                            AuthorizedActionTakers::MainGroup,
                        changing_authorized_action_takers_to_no_one_allowed: false,
                        changing_authorized_action_takers_to_contract_owner_allowed: false,
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
                    &vec![data_contract_create_serialized_transition.clone()],
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
                    ConsensusError::BasicError(BasicError::GroupMemberHasPowerOfZeroError(_)),
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
        fn test_data_contract_creation_with_group_with_member_with_too_big_power_should_error() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

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
                        authorized_to_change_authorized_action_takers:
                            AuthorizedActionTakers::MainGroup,
                        changing_authorized_action_takers_to_no_one_allowed: false,
                        changing_authorized_action_takers_to_contract_owner_allowed: false,
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
                    &vec![data_contract_create_serialized_transition.clone()],
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
                    ConsensusError::BasicError(BasicError::GroupMemberHasPowerOverLimitError(_)),
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
        fn test_data_contract_creation_with_group_with_member_with_power_over_required_should_error(
        ) {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

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
                        authorized_to_change_authorized_action_takers:
                            AuthorizedActionTakers::MainGroup,
                        changing_authorized_action_takers_to_no_one_allowed: false,
                        changing_authorized_action_takers_to_contract_owner_allowed: false,
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
                    &vec![data_contract_create_serialized_transition.clone()],
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
                    ConsensusError::BasicError(BasicError::GroupMemberHasPowerOverLimitError(_)),
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
        fn test_dcc_group_with_member_power_not_reaching_threshold() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

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
                        authorized_to_change_authorized_action_takers:
                            AuthorizedActionTakers::MainGroup,
                        changing_authorized_action_takers_to_no_one_allowed: false,
                        changing_authorized_action_takers_to_contract_owner_allowed: false,
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
                    &vec![data_contract_create_serialized_transition.clone()],
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
                    ConsensusError::BasicError(BasicError::GroupTotalPowerLessThanRequiredError(_)),
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
        fn test_dcc_group_with_non_unilateral_member_power_not_reaching_threshold() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

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
                        authorized_to_change_authorized_action_takers:
                            AuthorizedActionTakers::MainGroup,
                        changing_authorized_action_takers_to_no_one_allowed: false,
                        changing_authorized_action_takers_to_contract_owner_allowed: false,
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
                    &vec![data_contract_create_serialized_transition.clone()],
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
                        BasicError::GroupNonUnilateralMemberPowerHasLessThanRequiredPowerError(_)
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
    }
}
