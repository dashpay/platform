use dpp::block::block_info::BlockInfo;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;
use drive::state_transition_action::StateTransitionAction;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::data_contract_common::data_contract_reference_validation::validate_data_contract_references;
use crate::execution::validation::state_transition::state_transitions::data_contract_update::state::v0::DataContractUpdateStateTransitionStateValidationV0;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

pub(in crate::execution::validation::state_transition::state_transitions::data_contract_update) trait DataContractUpdateStateTransitionStateValidationV1
{
    fn validate_state_v1<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl DataContractUpdateStateTransitionStateValidationV1 for DataContractUpdateTransition {
    fn validate_state_v1<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let action = self.validate_state_v0::<C>(
            platform,
            block_info,
            validation_mode,
            execution_context,
            tx,
            platform_version,
        )?;

        if !action.is_valid() {
            return Ok(action);
        }

        // The updated contract may add document types or properties carrying
        // reference declarations, so they are re-validated on every update
        let reference_result = {
            let StateTransitionAction::DataContractUpdateAction(update_action) =
                action.data_as_borrowed()?
            else {
                return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "a valid data contract update state validation must contain an update action",
                )));
            };

            validate_data_contract_references(
                update_action.data_contract_ref(),
                platform.drive,
                block_info,
                execution_context,
                tx,
                platform_version,
            )?
        };

        if !reference_result.is_valid() {
            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                StateTransitionAction::BumpIdentityDataContractNonceAction(
                    BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(
                        self,
                    ),
                ),
                reference_result.errors,
            ));
        }

        Ok(action)
    }
}
