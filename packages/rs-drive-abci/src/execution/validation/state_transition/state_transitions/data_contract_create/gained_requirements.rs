//! The check contract create `transform_into_action` 1 (check_tx) and state validation 2 (a
//! block) both run, so the two refuse the same contracts.

use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::version::PlatformVersion;
use drive::state_transition_action::system::bump_identity_nonce_action::BumpIdentityNonceAction;
use drive::state_transition_action::StateTransitionAction;

use crate::error::execution::ExecutionError;
use crate::error::Error;

/// Every object a document type of a contract being registered reaches is one the contract
/// gains, so its `required` lists must be ones a document can meet, and each member of a type
/// must be required exactly when a list names it, as an update holds the objects it adds
/// (`DataContract::validate_gained_requirements`). A contract that breaks this is refused with
/// the owner's nonce bumped, as a contract the parse refuses is.
///
/// `result` is a contract create's action result: returned as it is when it is already
/// refused or the contract passes; a valid result must carry the create action.
pub(super) fn refuse_unmeetable_gained_requirements(
    transition: &DataContractCreateTransition,
    result: ConsensusValidationResult<StateTransitionAction>,
    platform_version: &PlatformVersion,
) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
    if !result.is_valid() {
        return Ok(result);
    }
    let Some(StateTransitionAction::DataContractCreateAction(action)) = result.data.as_ref() else {
        return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
            "a valid data contract create result must contain a create action",
        )));
    };

    let gained = action
        .data_contract_ref()
        .validate_gained_requirements(None, platform_version)?;
    if gained.is_valid() {
        return Ok(result);
    }
    Ok(ConsensusValidationResult::new_with_data_and_errors(
        StateTransitionAction::BumpIdentityNonceAction(
            BumpIdentityNonceAction::from_borrowed_data_contract_create_transition(transition),
        ),
        gained.errors,
    ))
}
