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
            Some(0) => self.validate_advanced_structure_v0(execution_context),
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
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::data_contract_create_transition::methods::DataContractCreateTransitionMethodsV0;
    use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
    use dpp::tests::json_document::json_document_to_contract_with_ids;
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
}
