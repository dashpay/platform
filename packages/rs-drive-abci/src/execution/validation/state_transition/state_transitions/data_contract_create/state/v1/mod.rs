use dpp::block::block_info::BlockInfo;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::system::bump_identity_nonce_action::BumpIdentityNonceAction;
use drive::state_transition_action::StateTransitionAction;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::data_contract_common::data_contract_reference_validation::validate_data_contract_references;
use crate::execution::validation::state_transition::state_transitions::data_contract_create::state::v0::DataContractCreateStateTransitionStateValidationV0;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

pub(in crate::execution::validation::state_transition::state_transitions::data_contract_create) trait DataContractCreateStateTransitionStateValidationV1
{
    fn validate_state_v1<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl DataContractCreateStateTransitionStateValidationV1 for DataContractCreateTransition {
    fn validate_state_v1<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let action = self.validate_state_v0::<C>(
            platform,
            block_info,
            validation_mode,
            tx,
            execution_context,
            platform_version,
        )?;

        if !action.is_valid() {
            return Ok(action);
        }

        let reference_result = {
            let StateTransitionAction::DataContractCreateAction(create_action) =
                action.data_as_borrowed()?
            else {
                return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "a valid data contract create state validation must contain a create action",
                )));
            };

            validate_data_contract_references(
                create_action.data_contract_ref(),
                platform.drive,
                block_info,
                execution_context,
                tx,
                platform_version,
            )?
        };

        if !reference_result.is_valid() {
            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                StateTransitionAction::BumpIdentityNonceAction(
                    BumpIdentityNonceAction::from_borrowed_data_contract_create_transition(self),
                ),
                reference_result.errors,
            ));
        }

        Ok(action)
    }
}
