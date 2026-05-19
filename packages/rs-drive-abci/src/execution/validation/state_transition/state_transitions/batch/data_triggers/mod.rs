use dpp::validation::SimpleValidationResult;
/// Data triggers implement custom validation logic for state transitions
/// that modifies documents in a specific data contract.
/// Data triggers can be assigned based on the data contract ID, document type, and action.
use drive::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;

use crate::error::Error;
use dpp::consensus::state::data_trigger::DataTriggerError;
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;

pub(super) use bindings::list::data_trigger_bindings_list;
pub(super) use context::DataTriggerExecutionContext;
pub(super) use executor::DataTriggerExecutor;

mod bindings;
mod context;
mod executor;
mod triggers;

/// Data trigger function pointer.
///
/// Returns the validation result and the `FeeResult` for whatever drive
/// reads (`query_documents`, `fetch_identity_balance`, etc.) the trigger
/// performed. The caller (`DataTriggerExecutor::validate_with_data_triggers`)
/// sums fees across all executed triggers and the outer batch-state
/// validator decides whether to bill them via the `transform_into_action`
/// version gate.
type DataTrigger = fn(
    &DocumentTransitionAction,
    &DataTriggerExecutionContext<'_>,
    &PlatformVersion,
) -> Result<(DataTriggerExecutionResult, FeeResult), Error>;

/// A type alias for a [SimpleValidationResult] with a [DataTriggerError] as the error type.
///
/// This type is used to represent the result of executing a data trigger on the blockchain. It contains either a
/// successful result or a `DataTriggerActionError`, indicating the failure of the trigger.
pub(super) type DataTriggerExecutionResult = SimpleValidationResult<DataTriggerError>;
