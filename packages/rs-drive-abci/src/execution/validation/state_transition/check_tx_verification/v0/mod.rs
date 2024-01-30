use crate::error::Error;
use crate::execution::types::execution_event::ExecutionEvent;
use crate::execution::validation::state_transition::transformer::StateTransitionActionTransformerV0;
use crate::platform_types::platform::PlatformRef;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;
use dpp::identity::state_transition::OptionallyAssetLockProved;
use dpp::prelude::ConsensusValidationResult;

use dpp::state_transition::{StateTransition};
use dpp::version::{DefaultForPlatformVersion, PlatformVersion};
use crate::execution::check_tx::CheckTxLevel;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::common::asset_lock::proof::verify_is_not_spent::AssetLockProofVerifyIsNotSpent;
use crate::execution::validation::state_transition::processor::process_state_transition;
use crate::execution::validation::state_transition::processor::v0::{StateTransitionSignatureValidationV0, StateTransitionStructureValidationV0};

/// A trait for validating state transitions within a blockchain.
pub(crate) trait StateTransitionCheckTxValidationV0 {
    /// This means we should do the full validation on check_tx
    fn requires_check_tx_full_validation(&self) -> bool;
}

impl StateTransitionCheckTxValidationV0 for StateTransition {
    fn requires_check_tx_full_validation(&self) -> bool {
        matches!(
            self,
            StateTransition::IdentityCreate(_) | StateTransition::IdentityTopUp(_)
        )
    }
}

pub(super) fn state_transition_to_execution_event_for_check_tx_v0<'a, C: CoreRPCLike>(
    platform: &'a PlatformRef<C>,
    state_transition: StateTransition,
    check_tx_level: CheckTxLevel,
    platform_version: &PlatformVersion,
) -> Result<ConsensusValidationResult<Option<ExecutionEvent<'a>>>, Error> {
    match check_tx_level {
        CheckTxLevel::FirstTimeCheck => {
            if state_transition.requires_check_tx_full_validation() {
                Ok(process_state_transition(platform, state_transition, None)?.map(Some))
            } else {
                // we need to validate the structure, the fees, and the signature
                let mut state_transition_execution_context =
                    StateTransitionExecutionContext::default_for_platform_version(
                        platform_version,
                    )?;

                let action = if state_transition.requires_state_to_validate_structure() {
                    let state_transition_action_result = state_transition.transform_into_action(
                        platform,
                        true,
                        &mut state_transition_execution_context,
                        None,
                    )?;
                    if !state_transition_action_result.is_valid_with_data() {
                        return Ok(
                            ConsensusValidationResult::<Option<ExecutionEvent>>::new_with_errors(
                                state_transition_action_result.errors,
                            ),
                        );
                    }
                    Some(state_transition_action_result.into_data()?)
                } else {
                    None
                };

                // Validating structure
                let result = state_transition.validate_structure(
                    &platform.into(),
                    action.as_ref(),
                    platform.state.current_protocol_version_in_consensus(),
                )?;
                if !result.is_valid() {
                    return Ok(
                        ConsensusValidationResult::<Option<ExecutionEvent>>::new_with_errors(
                            result.errors,
                        ),
                    );
                }

                let action = if state_transition
                    .requires_state_to_validate_identity_and_signatures()
                {
                    if let Some(action) = action {
                        Some(action)
                    } else {
                        let state_transition_action_result = state_transition
                            .transform_into_action(
                                platform,
                                true,
                                &mut state_transition_execution_context,
                                None,
                            )?;
                        if !state_transition_action_result.is_valid_with_data() {
                            return Ok(
                                    ConsensusValidationResult::<Option<ExecutionEvent>>::new_with_errors(
                                        state_transition_action_result.errors,
                                    ),
                                );
                        }
                        Some(state_transition_action_result.into_data()?)
                    }
                } else {
                    None
                };

                //
                let result = state_transition.validate_identity_and_signatures(
                    platform.drive,
                    action.as_ref(),
                    None,
                    &mut state_transition_execution_context,
                    platform_version,
                )?;
                // Validating signatures
                if !result.is_valid() {
                    return Ok(
                        ConsensusValidationResult::<Option<ExecutionEvent>>::new_with_errors(
                            result.errors,
                        ),
                    );
                }
                let maybe_identity = result.into_data()?;

                let action = if let Some(action) = action {
                    action
                } else {
                    let state_transition_action_result = state_transition.transform_into_action(
                        platform,
                        true,
                        &mut state_transition_execution_context,
                        None,
                    )?;
                    if !state_transition_action_result.is_valid_with_data() {
                        return Ok(
                            ConsensusValidationResult::<Option<ExecutionEvent>>::new_with_errors(
                                state_transition_action_result.errors,
                            ),
                        );
                    }
                    state_transition_action_result.into_data()?
                };

                let execution_event = ExecutionEvent::create_from_state_transition_action(
                    action,
                    maybe_identity,
                    platform.state.last_committed_block_epoch_ref(),
                    state_transition_execution_context,
                    platform_version,
                )?;

                Ok(
                    ConsensusValidationResult::<Option<ExecutionEvent>>::new_with_data(Some(
                        execution_event,
                    )),
                )
            }
        }
        CheckTxLevel::Recheck => {
            if let Some(asset_lock_proof) = state_transition.optional_asset_lock_proof() {
                // we should check that the asset lock is still valid
                let validation_result =
                    asset_lock_proof.verify_is_not_spent(platform, None, platform_version)?;

                if validation_result.is_valid() {
                    Ok(ConsensusValidationResult::<Option<ExecutionEvent>>::new_with_data(None))
                } else {
                    Ok(
                        ConsensusValidationResult::<Option<ExecutionEvent>>::new_with_errors(
                            validation_result.errors,
                        ),
                    )
                }
            } else {
                // TODO: We aren't calculating processing fees atm. We probably should reconsider this

                let mut state_transition_execution_context =
                    StateTransitionExecutionContext::default_for_platform_version(
                        platform_version,
                    )?;

                let state_transition_action_result = state_transition.transform_into_action(
                    platform,
                    true,
                    &mut state_transition_execution_context,
                    None,
                )?;

                if !state_transition_action_result.is_valid_with_data() {
                    return Ok(
                        ConsensusValidationResult::<Option<ExecutionEvent>>::new_with_errors(
                            state_transition_action_result.errors,
                        ),
                    );
                }
                let action = state_transition_action_result.into_data()?;

                let maybe_identity = platform.drive.fetch_identity_with_balance(
                    state_transition.owner_id().to_buffer(),
                    None,
                    platform_version,
                )?;

                let execution_event = ExecutionEvent::create_from_state_transition_action(
                    action,
                    maybe_identity,
                    platform.state.last_committed_block_epoch_ref(),
                    state_transition_execution_context,
                    platform_version,
                )?;

                Ok(
                    ConsensusValidationResult::<Option<ExecutionEvent>>::new_with_data(Some(
                        execution_event,
                    )),
                )
            }
        }
    }
}
