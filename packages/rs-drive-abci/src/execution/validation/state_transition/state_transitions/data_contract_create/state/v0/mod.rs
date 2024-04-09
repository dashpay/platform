use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use dpp::block::epoch::Epoch;

use dpp::consensus::state::data_contract::data_contract_already_present_error::DataContractAlreadyPresentError;
use dpp::consensus::state::state_error::StateError;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::data_contract_create_transition::accessors::DataContractCreateTransitionAccessorsV0;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::ProtocolError;

use crate::error::execution::ExecutionError;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::ValidationMode;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::contract::data_contract_create::DataContractCreateTransitionAction;
use drive::state_transition_action::system::bump_identity_nonce_action::BumpIdentityNonceAction;
use drive::state_transition_action::StateTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::data_contract_create) trait DataContractCreateStateTransitionStateValidationV0 {
    fn validate_state_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        validation_mode: ValidationMode,
        epoch: &Epoch,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;

    fn transform_into_action_v0<C: CoreRPCLike>(
        &self,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl DataContractCreateStateTransitionStateValidationV0 for DataContractCreateTransition {
    fn validate_state_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        validation_mode: ValidationMode,
        epoch: &Epoch,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let contract_fetch_info = platform.drive.get_contract_with_fetch_info_and_fee(
            self.data_contract().id().to_buffer(),
            Some(epoch),
            false,
            tx,
            platform_version,
        )?;

        let fee = contract_fetch_info.0.ok_or(Error::Execution(
            ExecutionError::CorruptedCodeExecution(
                "fee must exist in validate state for data contract create transition",
            ),
        ))?;

        // We add the cost for fetching the contract even if the contract doesn't exist or was in cache
        execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));

        // Data contract shouldn't exist
        if contract_fetch_info.1.is_some() {
            let bump_action = StateTransitionAction::BumpIdentityNonceAction(
                BumpIdentityNonceAction::from_borrowed_data_contract_create_transition(self),
            );

            Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action,
                vec![StateError::DataContractAlreadyPresentError(
                    DataContractAlreadyPresentError::new(self.data_contract().id().to_owned()),
                )
                .into()],
            ))
        } else {
            self.transform_into_action_v0::<C>(validation_mode, execution_context, platform_version)
        }
    }

    fn transform_into_action_v0<C: CoreRPCLike>(
        &self,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let mut validation_operations = vec![];

        // The transformation of the state transition into the state transition action will transform
        // The contract in serialized form into it's execution form
        let result = DataContractCreateTransitionAction::try_from_borrowed_transition(
            self,
            validation_mode.should_validate_contract_on_transform_into_action(),
            &mut validation_operations,
            platform_version,
        );

        execution_context.add_dpp_operations(validation_operations);

        // Return validation result if any consensus errors happened
        // during data contract validation
        match result {
            Err(ProtocolError::ConsensusError(consensus_error)) => {
                let bump_action = StateTransitionAction::BumpIdentityNonceAction(
                    BumpIdentityNonceAction::from_borrowed_data_contract_create_transition(self),
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
