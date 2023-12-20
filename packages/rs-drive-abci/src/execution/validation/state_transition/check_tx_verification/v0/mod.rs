use crate::error::Error;
use crate::execution::types::execution_event::ExecutionEvent;
use crate::execution::validation::state_transition::transformer::StateTransitionActionTransformerV0;
use crate::platform_types::platform::{PlatformRef, PlatformStateRef};
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;
use dpp::identity::PartialIdentity;
use dpp::prelude::ConsensusValidationResult;

use dpp::serialization::Signable;
use dpp::state_transition::{StateTransition};
use drive::state_transition_action::StateTransitionAction;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::{DefaultForPlatformVersion, PlatformVersion};
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use crate::error::execution::ExecutionError;
use crate::execution::check_tx::CheckTxLevel;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::common::validate_state_transition_identity_signed::{ValidateStateTransitionIdentitySignature};
use crate::execution::validation::state_transition::processor::process_state_transition;
use crate::execution::validation::state_transition::processor::v0::StateTransitionStructureValidationV0;
use crate::execution::validation::state_transition::state_transitions::identity_update::identity_and_signatures::v0::IdentityUpdateStateTransitionIdentityAndSignaturesValidationV0;
use crate::execution::validation::state_transition::state_transitions::identity_create::identity_and_signatures::v0::IdentityCreateStateTransitionIdentityAndSignaturesValidationV0;
use crate::execution::validation::state_transition::state_transitions::identity_top_up::identity_retrieval::v0::IdentityTopUpStateTransitionIdentityRetrievalV0;
pub(super) fn check_state_transition_v0<'a, C: CoreRPCLike>(
    platform: &'a PlatformRef<C>,
    state_transition: StateTransition,
    check_tx_level: CheckTxLevel,
    platform_version: &PlatformVersion,
) -> Result<ConsensusValidationResult<ExecutionEvent<'a>>, Error> {


    match check_tx_level {
        CheckTxLevel::FirstTimeCheck => {
            if state_transition.requires_check_tx_full_validation() {
                return process_state_transition(platform, state_transition, None);
            } else {
                // we need to validate the structure, the fees, and the signature
            }
        }
        CheckTxLevel::Recheck => {
            if let Some(asset_lock) = state_transition.asset_lock() {
                // we should check that the asset lock is still valid
                
            } else {

            }
        }
    }

    let mut state_transition_execution_context =
        StateTransitionExecutionContext::default_for_platform_version(platform_version)?;

    let action = if state_transition.requires_state_to_validate_structure() {
        let state_transition_action_result =
            state_transition.transform_into_action(platform, true, transaction)?;
        if !state_transition_action_result.is_valid_with_data() {
            return Ok(
                ConsensusValidationResult::<ExecutionEvent>::new_with_errors(
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
        return Ok(ConsensusValidationResult::<ExecutionEvent>::new_with_errors(result.errors));
    }

    let action = if state_transition.requires_state_to_validate_identity_and_signatures() {
        if let Some(action) = action {
            Some(action)
        } else {
            let state_transition_action_result =
                state_transition.transform_into_action(platform, true, transaction)?;
            if !state_transition_action_result.is_valid_with_data() {
                return Ok(
                    ConsensusValidationResult::<ExecutionEvent>::new_with_errors(
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
        transaction,
        &mut state_transition_execution_context,
        platform_version,
    )?;
    // Validating signatures
    if !result.is_valid() {
        return Ok(ConsensusValidationResult::<ExecutionEvent>::new_with_errors(result.errors));
    }
    let maybe_identity = result.into_data()?;

    // Validating state
    let result = state_transition.validate_state(action, platform, transaction)?;

    result.map_result(|action| {
        ExecutionEvent::create_from_state_transition_action(
            action,
            maybe_identity,
            platform.state.epoch_ref(),
            platform_version,
        )
    })
}