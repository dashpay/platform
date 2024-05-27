use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use dpp::block::epoch::Epoch;

use dpp::consensus::basic::document::DataContractNotPresentError;
use dpp::consensus::basic::BasicError;

use dpp::data_contract::accessors::v0::DataContractV0Getters;

use dpp::data_contract::validate_update::DataContractUpdateValidationMethodsV0;

use dpp::prelude::ConsensusValidationResult;
use dpp::ProtocolError;

use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::version::PlatformVersion;

use crate::error::execution::ExecutionError;
use crate::execution::validation::state_transition::ValidationMode;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::contract::data_contract_update::DataContractUpdateTransitionAction;
use drive::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;

use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use drive::state_transition_action::StateTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::data_contract_update) trait DataContractUpdateStateTransitionStateValidationV0 {
    fn validate_state_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        validation_mode: ValidationMode,
        epoch: &Epoch,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;

    fn transform_into_action_v0(
        &self,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl DataContractUpdateStateTransitionStateValidationV0 for DataContractUpdateTransition {
    fn validate_state_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        validation_mode: ValidationMode,
        epoch: &Epoch,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let action =
            self.transform_into_action_v0(validation_mode, execution_context, platform_version)?;

        if !action.is_valid() {
            return Ok(action);
        }

        let state_transition_action = action.data.as_ref().ok_or(Error::Execution(
            ExecutionError::CorruptedCodeExecution(
                "we should always have an action at this point in data contract update",
            ),
        ))?;

        let new_data_contract = match state_transition_action {
            StateTransitionAction::DataContractUpdateAction(action) => {
                Some(action.data_contract_ref())
            }
            _ => None,
        }
        .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
            "we should always have a data contract at this point in data contract update",
        )))?;

        let drive = platform.drive;

        // Check previous data contract already exists in the state
        // Failure (contract does not exist): Keep ST and transform it to a nonce bump action.
        // How: A user pushed an update for a data contract that didn’t exist.
        // Note: Existing in the state can also mean that it exists in the current block state, meaning that the contract was inserted in the same block with a previous transition.

        // Data contract should exist
        let add_to_cache_if_pulled = validation_mode.can_alter_cache();

        let data_contract_fetch_info = drive.get_contract_with_fetch_info_and_fee(
            new_data_contract.id().to_buffer(),
            Some(epoch),
            add_to_cache_if_pulled,
            tx,
            platform_version,
        )?;

        let fee = data_contract_fetch_info.0.ok_or(Error::Execution(
            ExecutionError::CorruptedCodeExecution(
                "fee must exist in validate state for data contract update transition",
            ),
        ))?;

        // We add the cost for fetching the contract even if the contract doesn't exist or was in cache
        execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));

        // Data contract should exist
        let Some(contract_fetch_info) = data_contract_fetch_info.1 else {
            let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(
                    self,
                ),
            );

            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action,
                vec![
                    BasicError::DataContractNotPresentError(DataContractNotPresentError::new(
                        new_data_contract.id(),
                    ))
                    .into(),
                ],
            ));
        };

        let old_data_contract = &contract_fetch_info.contract;

        let validation_result =
            old_data_contract.validate_update(new_data_contract, platform_version)?;

        if !validation_result.is_valid() {
            let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(
                    self,
                ),
            );

            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action,
                validation_result.errors,
            ));
        }

        Ok(action)
    }

    fn transform_into_action_v0(
        &self,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let mut validation_operations = vec![];

        let result = DataContractUpdateTransitionAction::try_from_borrowed_transition(
            self,
            validation_mode.should_fully_validate_contract_on_transform_into_action(),
            &mut validation_operations,
            platform_version,
        );

        execution_context.add_dpp_operations(validation_operations);

        // Return validation result if any consensus errors happened
        // during data contract validation
        match result {
            Err(ProtocolError::ConsensusError(consensus_error)) => {
                let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                    BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(self),
                );

                Ok(ConsensusValidationResult::new_with_data_and_errors(
                    bump_action,
                    vec![*consensus_error],
                ))
            }
            Err(protocol_error) => Err(protocol_error.into()),
            Ok(create_action) => {
                let action: StateTransitionAction = create_action.into();
                Ok(action.into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use assert_matches::assert_matches;
    use dpp::block::block_info::BlockInfo;
    use dpp::consensus::ConsensusError;
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    use dpp::data_contract::errors::DataContractError;
    use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
    use dpp::platform_value::Value;
    use dpp::prelude::IdentityNonce;
    use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransitionV0;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use drive::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceActionAccessorsV0;
    use platform_version::{DefaultForPlatformVersion, TryIntoPlatformVersioned};

    mod validate_state_v0 {
        use super::*;

        #[test]
        fn should_return_invalid_result_when_transform_into_action_failed() {
            let platform_version = PlatformVersion::latest();
            let identity_contract_nonce = IdentityNonce::default();

            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let data_contract = get_data_contract_fixture(
                None,
                identity_contract_nonce,
                platform_version.protocol_version,
            )
            .data_contract_owned();

            let identity_id = data_contract.owner_id();
            let data_contract_id = data_contract.id();

            let mut data_contract_for_serialization = data_contract
                .try_into_platform_versioned(platform_version)
                .expect("failed to convert data contract");

            // Make the contract invalid
            let DataContractInSerializationFormat::V0(ref mut contract) =
                data_contract_for_serialization;

            contract
                .document_schemas
                .insert("invalidType".to_string(), Value::Null);

            let transition: DataContractUpdateTransition = DataContractUpdateTransitionV0 {
                identity_contract_nonce,
                data_contract: data_contract_for_serialization,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: Default::default(),
            }
            .into();

            let mut execution_context =
                StateTransitionExecutionContext::default_for_platform_version(platform_version)
                    .expect("failed to create execution context");

            let state = platform.state.load_full();

            let platform_ref = PlatformRef {
                drive: &platform.drive,
                state: &state,
                config: &platform.config,
                core_rpc: &platform.core_rpc,
            };

            let result = transition
                .validate_state_v0::<MockCoreRPCLike>(
                    &platform_ref,
                    ValidationMode::Validator,
                    &Epoch::default(),
                    &mut execution_context,
                    None,
                    platform_version,
                )
                .expect("failed to validate advanced structure");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::ContractError(
                        DataContractError::InvalidContractStructure(message)
                    )
                )] if message == "document schema must be an object: structure error: value is not a map"
            );

            assert_matches!(
                result.data,
                Some(StateTransitionAction::BumpIdentityDataContractNonceAction(action))
                if action.identity_id() == identity_id && action.identity_contract_nonce() == identity_contract_nonce && action.data_contract_id() == data_contract_id
            );

            // We have tons of operations here so not sure we want to assert all of them
            assert!(!execution_context.operations_slice().is_empty());
        }

        #[test]
        fn should_return_invalid_result_when_data_contract_does_not_exist() {
            let platform_version = PlatformVersion::latest();
            let identity_contract_nonce = IdentityNonce::default();

            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let data_contract = get_data_contract_fixture(
                None,
                identity_contract_nonce,
                platform_version.protocol_version,
            )
            .data_contract_owned();

            let identity_id = data_contract.owner_id();
            let data_contract_id = data_contract.id();

            let data_contract_for_serialization = data_contract
                .try_into_platform_versioned(platform_version)
                .expect("failed to convert data contract");

            let transition: DataContractUpdateTransition = DataContractUpdateTransitionV0 {
                identity_contract_nonce,
                data_contract: data_contract_for_serialization,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: Default::default(),
            }
            .into();

            let mut execution_context =
                StateTransitionExecutionContext::default_for_platform_version(platform_version)
                    .expect("failed to create execution context");

            let state = platform.state.load_full();

            let platform_ref = PlatformRef {
                drive: &platform.drive,
                state: &state,
                config: &platform.config,
                core_rpc: &platform.core_rpc,
            };

            let result = transition
                .validate_state_v0::<MockCoreRPCLike>(
                    &platform_ref,
                    ValidationMode::Validator,
                    &Epoch::default(),
                    &mut execution_context,
                    None,
                    platform_version,
                )
                .expect("failed to validate advanced structure");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::DataContractNotPresentError(e)
                )] if e.data_contract_id() == data_contract_id
            );

            assert_matches!(
                result.data,
                Some(StateTransitionAction::BumpIdentityDataContractNonceAction(action))
                if action.identity_id() == identity_id && action.identity_contract_nonce() == identity_contract_nonce && action.data_contract_id() == data_contract_id
            );

            // We have tons of operations here so not sure we want to assert all of them
            assert!(!execution_context.operations_slice().is_empty());
        }

        #[test]
        fn should_return_invalid_result_when_new_data_contract_has_incompatible_changes() {
            let platform_version = PlatformVersion::latest();
            let identity_contract_nonce = IdentityNonce::default();

            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let data_contract = get_data_contract_fixture(
                None,
                identity_contract_nonce,
                platform_version.protocol_version,
            )
            .data_contract_owned();

            platform
                .drive
                .apply_contract(
                    &data_contract,
                    BlockInfo::default(),
                    true,
                    None,
                    None,
                    platform_version,
                )
                .expect("failed to apply contract");

            let identity_id = data_contract.owner_id();
            let data_contract_id = data_contract.id();

            let data_contract_for_serialization = data_contract
                .try_into_platform_versioned(platform_version)
                .expect("failed to convert data contract");

            let transition: DataContractUpdateTransition = DataContractUpdateTransitionV0 {
                identity_contract_nonce,
                data_contract: data_contract_for_serialization,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: Default::default(),
            }
            .into();

            let mut execution_context =
                StateTransitionExecutionContext::default_for_platform_version(platform_version)
                    .expect("failed to create execution context");

            let state = platform.state.load_full();

            let platform_ref = PlatformRef {
                drive: &platform.drive,
                state: &state,
                config: &platform.config,
                core_rpc: &platform.core_rpc,
            };

            let result = transition
                .validate_state_v0::<MockCoreRPCLike>(
                    &platform_ref,
                    ValidationMode::Validator,
                    &Epoch::default(),
                    &mut execution_context,
                    None,
                    platform_version,
                )
                .expect("failed to validate advanced structure");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::InvalidDataContractVersionError(e)
                )] if e.expected_version() == 2 && e.version() == 1
            );

            assert_matches!(
                result.data,
                Some(StateTransitionAction::BumpIdentityDataContractNonceAction(action))
                if action.identity_id() == identity_id && action.identity_contract_nonce() == identity_contract_nonce && action.data_contract_id() == data_contract_id
            );

            // We have tons of operations here so not sure we want to assert all of them
            assert!(!execution_context.operations_slice().is_empty());
        }

        #[test]
        fn should_pass_when_contract_exists_and_update_is_compatible() {
            let platform_version = PlatformVersion::latest();
            let identity_contract_nonce = IdentityNonce::default();

            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut data_contract = get_data_contract_fixture(
                None,
                identity_contract_nonce,
                platform_version.protocol_version,
            )
            .data_contract_owned();

            platform
                .drive
                .apply_contract(
                    &data_contract,
                    BlockInfo::default(),
                    true,
                    None,
                    None,
                    platform_version,
                )
                .expect("failed to apply contract");

            let data_contract_id = data_contract.id();

            data_contract.set_version(2);

            let data_contract_for_serialization = data_contract
                .try_into_platform_versioned(platform_version)
                .expect("failed to convert data contract");

            let transition: DataContractUpdateTransition = DataContractUpdateTransitionV0 {
                identity_contract_nonce,
                data_contract: data_contract_for_serialization,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: Default::default(),
            }
            .into();

            let mut execution_context =
                StateTransitionExecutionContext::default_for_platform_version(platform_version)
                    .expect("failed to create execution context");

            let state = platform.state.load_full();

            let platform_ref = PlatformRef {
                drive: &platform.drive,
                state: &state,
                config: &platform.config,
                core_rpc: &platform.core_rpc,
            };

            let result = transition
                .validate_state_v0::<MockCoreRPCLike>(
                    &platform_ref,
                    ValidationMode::Validator,
                    &Epoch::default(),
                    &mut execution_context,
                    None,
                    platform_version,
                )
                .expect("failed to validate advanced structure");

            assert_matches!(result.errors.as_slice(), []);

            assert_matches!(
                result.data,
                Some(StateTransitionAction::DataContractUpdateAction(action))
                if action.data_contract_ref().id() == data_contract_id
            );

            // We have tons of operations here so not sure we want to assert all of them
            assert!(!execution_context.operations_slice().is_empty());
        }
    }

    mod transform_into_action_v0 {
        use super::*;

        #[test]
        fn should_return_invalid_result_when_new_data_contract_is_not_valid() {
            let platform_version = PlatformVersion::latest();
            let identity_contract_nonce = IdentityNonce::default();

            let data_contract = get_data_contract_fixture(
                None,
                identity_contract_nonce,
                platform_version.protocol_version,
            )
            .data_contract_owned();

            let identity_id = data_contract.owner_id();
            let data_contract_id = data_contract.id();

            let mut data_contract_for_serialization = data_contract
                .try_into_platform_versioned(platform_version)
                .expect("failed to convert data contract");

            // Make the contract invalid
            let DataContractInSerializationFormat::V0(ref mut contract) =
                data_contract_for_serialization;

            contract
                .document_schemas
                .insert("invalidType".to_string(), Value::Null);

            let transition: DataContractUpdateTransition = DataContractUpdateTransitionV0 {
                identity_contract_nonce,
                data_contract: data_contract_for_serialization,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: Default::default(),
            }
            .into();

            let mut execution_context =
                StateTransitionExecutionContext::default_for_platform_version(platform_version)
                    .expect("failed to create execution context");

            let result = transition
                .transform_into_action_v0(
                    ValidationMode::Validator,
                    &mut execution_context,
                    platform_version,
                )
                .expect("failed to validate advanced structure");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::ContractError(
                        DataContractError::InvalidContractStructure(message)
                    )
                )] if message == "document schema must be an object: structure error: value is not a map"
            );

            assert_matches!(
                result.data,
                Some(StateTransitionAction::BumpIdentityDataContractNonceAction(action))
                if action.identity_id() == identity_id && action.identity_contract_nonce() == identity_contract_nonce && action.data_contract_id() == data_contract_id
            );

            // We have tons of operations here so not sure we want to assert all of them
            assert!(!execution_context.operations_slice().is_empty());
        }

        #[test]
        fn should_pass_when_new_data_contract_is_valid() {
            let platform_version = PlatformVersion::latest();
            let identity_contract_nonce = IdentityNonce::default();

            let data_contract = get_data_contract_fixture(
                None,
                identity_contract_nonce,
                platform_version.protocol_version,
            )
            .data_contract_owned();

            let data_contract_id = data_contract.id();

            let data_contract_for_serialization = data_contract
                .try_into_platform_versioned(platform_version)
                .expect("failed to convert data contract");

            let transition: DataContractUpdateTransition = DataContractUpdateTransitionV0 {
                identity_contract_nonce,
                data_contract: data_contract_for_serialization,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: Default::default(),
            }
            .into();

            let mut execution_context =
                StateTransitionExecutionContext::default_for_platform_version(platform_version)
                    .expect("failed to create execution context");

            let result = transition
                .transform_into_action_v0(
                    ValidationMode::Validator,
                    &mut execution_context,
                    platform_version,
                )
                .expect("failed to validate advanced structure");

            assert_matches!(result.errors.as_slice(), []);

            assert_matches!(
                result.data,
                Some(StateTransitionAction::DataContractUpdateAction(action)) if action.data_contract_ref().id() == data_contract_id
            );

            // We have tons of operations here so not sure we want to assert all of them
            assert!(!execution_context.operations_slice().is_empty());
        }
    }
}
