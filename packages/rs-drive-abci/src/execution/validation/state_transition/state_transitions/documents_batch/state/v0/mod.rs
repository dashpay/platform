use dpp::block::block_info::BlockInfo;
use dpp::consensus::ConsensusError;
use dpp::consensus::state::state_error::StateError;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::documents_batch_transition::DocumentsBatchTransition;
use dpp::state_transition::StateTransitionLike;
use drive::state_transition_action::StateTransitionAction;
use dpp::version::{DefaultForPlatformVersion, PlatformVersion};
use drive::grovedb::TransactionArg;
use drive::state_transition_action::document::documents_batch::document_transition::DocumentTransitionAction;
use drive::state_transition_action::document::documents_batch::DocumentsBatchTransitionAction;
use drive::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;
use crate::error::Error;
use crate::error::execution::ExecutionError;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::documents_batch::action_validation::document_create_transition_action::DocumentCreateTransitionActionValidation;
use crate::execution::validation::state_transition::documents_batch::action_validation::document_delete_transition_action::DocumentDeleteTransitionActionValidation;
use crate::execution::validation::state_transition::documents_batch::action_validation::document_replace_transition_action::DocumentReplaceTransitionActionValidation;
use crate::execution::validation::state_transition::documents_batch::data_triggers::{data_trigger_bindings_list, DataTriggerExecutionContext, DataTriggerExecutor};
use crate::platform_types::platform::{PlatformStateRef};
use crate::execution::validation::state_transition::state_transitions::documents_batch::transformer::v0::DocumentsBatchTransitionTransformerV0;
use crate::execution::validation::state_transition::ValidationMode;

mod data_triggers;
pub mod fetch_documents;

pub(in crate::execution::validation::state_transition::state_transitions::documents_batch) trait DocumentsBatchStateTransitionStateValidationV0
{
    fn validate_state_v0(
        &self,
        action: DocumentsBatchTransitionAction,
        platform: &PlatformStateRef,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;

    fn transform_into_action_v0(
        &self,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl DocumentsBatchStateTransitionStateValidationV0 for DocumentsBatchTransition {
    fn validate_state_v0(
        &self,
        mut state_transition_action: DocumentsBatchTransitionAction,
        platform: &PlatformStateRef,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let mut validation_result = ConsensusValidationResult::<StateTransitionAction>::new();

        let state_transition_execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)?;

        let owner_id = state_transition_action.owner_id();

        let mut validated_transitions = vec![];

        let data_trigger_bindings = if platform.config.execution.use_document_triggers {
            data_trigger_bindings_list(platform_version)?
        } else {
            vec![]
        };

        // Next we need to validate the structure of all actions (this means with the data contract)
        for transition in state_transition_action.transitions_take() {
            let transition_validation_result = match &transition {
                DocumentTransitionAction::CreateAction(create_action) => create_action
                    .validate_state(platform, owner_id, transaction, platform_version)?,
                DocumentTransitionAction::ReplaceAction(replace_action) => replace_action
                    .validate_state(platform, owner_id, transaction, platform_version)?,
                DocumentTransitionAction::DeleteAction(delete_action) => delete_action
                    .validate_state(platform, owner_id, transaction, platform_version)?,
                DocumentTransitionAction::BumpIdentityDataContractNonce(..) => {
                    return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "we should never start with a bump identity data contract nonce",
                    )));
                }
            };

            if !transition_validation_result.is_valid() {
                // If a state transition isn't valid we still need to bump the identity data contract nonce
                validation_result.add_errors(transition_validation_result.errors);
                validated_transitions.push(
                    DocumentTransitionAction::BumpIdentityDataContractNonce(
                        BumpIdentityDataContractNonceAction::from_document_base_transition_action(
                            transition.base_owned().ok_or(Error::Execution(
                                ExecutionError::CorruptedCodeExecution(
                                    "base should always exist on transition",
                                ),
                            ))?,
                            owner_id,
                            state_transition_action.user_fee_increase(),
                        )?,
                    ),
                );
            } else if platform.config.execution.use_document_triggers {
                // we should also validate document triggers
                let data_trigger_execution_context = DataTriggerExecutionContext {
                    platform,
                    transaction,
                    owner_id: &self.owner_id(),
                    state_transition_execution_context: &state_transition_execution_context,
                };
                let data_trigger_execution_result = transition.validate_with_data_triggers(
                    &data_trigger_bindings,
                    &data_trigger_execution_context,
                    platform_version,
                )?;

                if !data_trigger_execution_result.is_valid() {
                    tracing::debug!(
                        "{:?} state transition data trigger was not valid, errors are {:?}",
                        transition,
                        data_trigger_execution_result.errors,
                    );
                    // If a state transition isn't valid because of data triggers we still need
                    // to bump the identity data contract nonce
                    let consensus_errors: Vec<ConsensusError> = data_trigger_execution_result
                        .errors
                        .into_iter()
                        .map(|e| ConsensusError::StateError(StateError::DataTriggerError(e)))
                        .collect();
                    validation_result.add_errors(consensus_errors);
                    validated_transitions
                        .push(DocumentTransitionAction::BumpIdentityDataContractNonce(
                        BumpIdentityDataContractNonceAction::from_document_base_transition_action(
                            transition.base_owned().ok_or(Error::Execution(
                                ExecutionError::CorruptedCodeExecution(
                                    "base should always exist on transition",
                                ),
                            ))?,
                            owner_id,
                            state_transition_action.user_fee_increase(),
                        )?,
                    ));
                } else {
                    validated_transitions.push(transition);
                }
            } else {
                validated_transitions.push(transition);
            }
        }

        state_transition_action.set_transitions(validated_transitions);

        validation_result.set_data(state_transition_action.into());

        Ok(validation_result)
    }

    fn transform_into_action_v0(
        &self,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;

        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)?;

        let validation_result = self.try_into_action_v0(
            platform,
            block_info,
            validation_mode.should_validate_document_valid_against_state(),
            tx,
            &mut execution_context,
        )?;

        Ok(validation_result.map(Into::into))
    }
}
