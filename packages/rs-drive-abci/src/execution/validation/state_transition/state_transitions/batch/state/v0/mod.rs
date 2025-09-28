use dpp::block::block_info::BlockInfo;
use dpp::consensus::ConsensusError;
use dpp::consensus::state::state_error::StateError;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::state_transition::StateTransitionLike;
use drive::state_transition_action::StateTransitionAction;
use dpp::version::{DefaultForPlatformVersion, PlatformVersion};
use drive::drive::subscriptions::DriveSubscriptionFilter;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use drive::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;
use drive::state_transition_action::batch::batched_transition::token_transition::TokenTransitionAction;
use drive::state_transition_action::batch::BatchTransitionAction;
use drive::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;
use drive::state_transition_action::transform_to_state_transition_action_result::TransformToStateTransitionActionResult;
use crate::error::Error;
use crate::error::execution::ExecutionError;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::batch::action_validation::document::document_create_transition_action::DocumentCreateTransitionActionValidation;
use crate::execution::validation::state_transition::batch::action_validation::document::document_delete_transition_action::DocumentDeleteTransitionActionValidation;
use crate::execution::validation::state_transition::batch::action_validation::document::document_purchase_transition_action::DocumentPurchaseTransitionActionValidation;
use crate::execution::validation::state_transition::batch::action_validation::document::document_replace_transition_action::DocumentReplaceTransitionActionValidation;
use crate::execution::validation::state_transition::batch::action_validation::document::document_transfer_transition_action::DocumentTransferTransitionActionValidation;
use crate::execution::validation::state_transition::batch::action_validation::document::document_update_price_transition_action::DocumentUpdatePriceTransitionActionValidation;
use crate::execution::validation::state_transition::batch::action_validation::token::token_burn_transition_action::TokenBurnTransitionActionValidation;
use crate::execution::validation::state_transition::batch::action_validation::token::token_claim_transition_action::TokenClaimTransitionActionValidation;
use crate::execution::validation::state_transition::batch::action_validation::token::token_config_update_transition_action::TokenConfigUpdateTransitionActionValidation;
use crate::execution::validation::state_transition::batch::action_validation::token::token_destroy_frozen_funds_transition_action::TokenDestroyFrozenFundsTransitionActionValidation;
use crate::execution::validation::state_transition::batch::action_validation::token::token_direct_purchase_transition_action::TokenDirectPurchaseTransitionActionValidation;
use crate::execution::validation::state_transition::batch::action_validation::token::token_emergency_action_transition_action::TokenEmergencyActionTransitionActionValidation;
use crate::execution::validation::state_transition::batch::action_validation::token::token_freeze_transition_action::TokenFreezeTransitionActionValidation;
use crate::execution::validation::state_transition::batch::action_validation::token::token_mint_transition_action::TokenMintTransitionActionValidation;
use crate::execution::validation::state_transition::batch::action_validation::token::token_set_price_for_direct_purchase_transition_action::TokenSetPriceForDirectPurchaseTransitionActionValidation;
use crate::execution::validation::state_transition::batch::action_validation::token::token_transfer_transition_action::TokenTransferTransitionActionValidation;
use crate::execution::validation::state_transition::batch::action_validation::token::token_unfreeze_transition_action::TokenUnfreezeTransitionActionValidation;
use crate::execution::validation::state_transition::batch::data_triggers::{data_trigger_bindings_list, DataTriggerExecutionContext, DataTriggerExecutor};
use crate::platform_types::platform::{PlatformStateRef};
use crate::execution::validation::state_transition::state_transitions::batch::transformer::v0::BatchTransitionTransformerV0;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;

mod data_triggers;
pub mod fetch_contender;
pub mod fetch_documents;

pub(in crate::execution::validation::state_transition::state_transitions::batch) trait DocumentsBatchStateTransitionStateValidationV0
{
    fn validate_state_v0<'a>(
        &self,
        action: BatchTransitionAction,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        // These are the filters that have already shown that this transition is a match
        passing_filters_for_transition: &[&'a DriveSubscriptionFilter],
        // These are the filters that might still pass, if the original passes
        requiring_original_filters_for_transition: &[&'a DriveSubscriptionFilter],
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<TransformToStateTransitionActionResult<'a>>, Error>;

    fn transform_into_action_v0<'a>(
        &self,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        // These are the filters that have already shown that this transition is a match
        passing_filters_for_transition: &[&'a DriveSubscriptionFilter],
        // These are the filters that might still pass, if the original passes
        requiring_original_filters_for_transition: &[&'a DriveSubscriptionFilter],
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<TransformToStateTransitionActionResult<'a>>, Error>;
}

impl DocumentsBatchStateTransitionStateValidationV0 for BatchTransition {
    fn validate_state_v0<'a>(
        &self,
        mut state_transition_action: BatchTransitionAction,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        // These are the filters that have already shown that this transition is a match
        passing_filters_for_transition: &[&'a DriveSubscriptionFilter],
        // These are the filters that might still pass, if the original passes
        requiring_original_filters_for_transition: &[&'a DriveSubscriptionFilter],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<TransformToStateTransitionActionResult<'a>>, Error> {
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
                BatchedTransitionAction::DocumentAction(document_action) => match document_action {
                    DocumentTransitionAction::CreateAction(create_action) => create_action
                        .validate_state(
                            platform,
                            owner_id,
                            block_info,
                            execution_context,
                            transaction,
                            platform_version,
                        )?,
                    DocumentTransitionAction::ReplaceAction(replace_action) => replace_action
                        .validate_state(
                            platform,
                            owner_id,
                            block_info,
                            execution_context,
                            transaction,
                            platform_version,
                        )?,
                    DocumentTransitionAction::TransferAction(transfer_action) => transfer_action
                        .validate_state(
                            platform,
                            owner_id,
                            block_info,
                            execution_context,
                            transaction,
                            platform_version,
                        )?,
                    DocumentTransitionAction::DeleteAction(delete_action) => delete_action
                        .validate_state(
                            platform,
                            owner_id,
                            block_info,
                            execution_context,
                            transaction,
                            platform_version,
                        )?,
                    DocumentTransitionAction::UpdatePriceAction(update_price_action) => {
                        update_price_action.validate_state(
                            platform,
                            owner_id,
                            block_info,
                            execution_context,
                            transaction,
                            platform_version,
                        )?
                    }
                    DocumentTransitionAction::PurchaseAction(purchase_action) => purchase_action
                        .validate_state(
                            platform,
                            owner_id,
                            block_info,
                            execution_context,
                            transaction,
                            platform_version,
                        )?,
                },
                BatchedTransitionAction::TokenAction(token_action) => match token_action {
                    TokenTransitionAction::BurnAction(burn_action) => burn_action.validate_state(
                        platform,
                        owner_id,
                        block_info,
                        execution_context,
                        transaction,
                        platform_version,
                    )?,
                    TokenTransitionAction::MintAction(mint_action) => mint_action.validate_state(
                        platform,
                        owner_id,
                        block_info,
                        execution_context,
                        transaction,
                        platform_version,
                    )?,
                    TokenTransitionAction::TransferAction(transfer_action) => transfer_action
                        .validate_state(
                            platform,
                            owner_id,
                            block_info,
                            execution_context,
                            transaction,
                            platform_version,
                        )?,
                    TokenTransitionAction::FreezeAction(freeze_action) => freeze_action
                        .validate_state(
                            platform,
                            owner_id,
                            block_info,
                            execution_context,
                            transaction,
                            platform_version,
                        )?,
                    TokenTransitionAction::UnfreezeAction(unfreeze_action) => unfreeze_action
                        .validate_state(
                            platform,
                            owner_id,
                            block_info,
                            execution_context,
                            transaction,
                            platform_version,
                        )?,
                    TokenTransitionAction::EmergencyActionAction(emergency_action_action) => {
                        emergency_action_action.validate_state(
                            platform,
                            owner_id,
                            block_info,
                            execution_context,
                            transaction,
                            platform_version,
                        )?
                    }
                    TokenTransitionAction::DestroyFrozenFundsAction(
                        destroy_frozen_funds_action,
                    ) => destroy_frozen_funds_action.validate_state(
                        platform,
                        owner_id,
                        block_info,
                        execution_context,
                        transaction,
                        platform_version,
                    )?,
                    TokenTransitionAction::ConfigUpdateAction(config_update_action) => {
                        config_update_action.validate_state(
                            platform,
                            owner_id,
                            block_info,
                            execution_context,
                            transaction,
                            platform_version,
                        )?
                    }
                    TokenTransitionAction::ClaimAction(claim_action) => claim_action
                        .validate_state(
                            platform,
                            owner_id,
                            block_info,
                            execution_context,
                            transaction,
                            platform_version,
                        )?,
                    TokenTransitionAction::DirectPurchaseAction(direct_purchase_action) => {
                        direct_purchase_action.validate_state(
                            platform,
                            owner_id,
                            block_info,
                            execution_context,
                            transaction,
                            platform_version,
                        )?
                    }
                    TokenTransitionAction::SetPriceForDirectPurchaseAction(
                        set_price_for_direct_purchase_action,
                    ) => set_price_for_direct_purchase_action.validate_state(
                        platform,
                        owner_id,
                        block_info,
                        execution_context,
                        transaction,
                        platform_version,
                    )?,
                },
                BatchedTransitionAction::BumpIdentityDataContractNonce(_) => {
                    return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "we should never start with a bump identity data contract nonce",
                    )));
                }
            };

            if !transition_validation_result.is_valid() {
                // If a state transition isn't valid we still need to bump the identity data contract nonce
                validation_result.add_errors(transition_validation_result.errors);
                validated_transitions.push(BatchedTransitionAction::BumpIdentityDataContractNonce(
                    BumpIdentityDataContractNonceAction::try_from_batched_transition_action(
                        transition,
                        owner_id,
                        state_transition_action.user_fee_increase(),
                    )?,
                ));
            } else if platform.config.execution.use_document_triggers {
                if let BatchedTransitionAction::DocumentAction(document_transition) = &transition {
                    // we should also validate document triggers
                    let data_trigger_execution_context = DataTriggerExecutionContext {
                        platform,
                        transaction,
                        owner_id: &self.owner_id(),
                        state_transition_execution_context: &state_transition_execution_context,
                    };
                    let data_trigger_execution_result = document_transition
                        .validate_with_data_triggers(
                            &data_trigger_bindings,
                            &data_trigger_execution_context,
                            platform_version,
                        )?;

                    if !data_trigger_execution_result.is_valid() {
                        // If a state transition isn't valid because of data triggers we still need
                        // to bump the identity data contract nonce
                        let consensus_errors: Vec<ConsensusError> = data_trigger_execution_result
                            .errors
                            .into_iter()
                            .map(|e| ConsensusError::StateError(StateError::DataTriggerError(e)))
                            .collect();
                        validation_result.add_errors(consensus_errors);
                        validated_transitions
                            .push(BatchedTransitionAction::BumpIdentityDataContractNonce(
                                BumpIdentityDataContractNonceAction::from_borrowed_document_base_transition_action(
                                    document_transition.base(),
                                    owner_id,
                                    state_transition_action.user_fee_increase(),
                                ),
                            ));
                    } else {
                        validated_transitions.push(transition);
                    }
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

    fn transform_into_action_v0<'a>(
        &self,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        // These are the filters that have already shown that this transition is a match
        passing_filters_for_transition: &[&'a DriveSubscriptionFilter],
        // These are the filters that might still pass, if the original passes
        requiring_original_filters_for_transition: &[&'a DriveSubscriptionFilter],
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<TransformToStateTransitionActionResult<'a>>, Error> {
        let platform_version = platform.state.current_platform_version()?;

        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)?;

        let validation_result = self.try_into_action_v0(
            platform,
            block_info,
            validation_mode.should_validate_batch_valid_against_state(),
            passing_filters_for_transition,
            requiring_original_filters_for_transition,
            tx,
            &mut execution_context,
        )?;

        Ok(validation_result.map(Into::into))
    }
}
