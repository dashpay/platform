use dpp::validation::SimpleValidationResult;
/// Data triggers implement custom validation logic for state transitions
/// that modifies documents in a specific data contract.
/// Data triggers can be assigned based on the data contract ID, document type, and action.
use drive::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;

use crate::error::Error;
use dpp::consensus::state::data_trigger::DataTriggerError;
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
/// Takes a mutable `DataTriggerExecutionContext` so `_v1` triggers can
/// call `context.state_transition_execution_context.add_operation(...)`
/// directly to bill their drive reads. `_v0` triggers don't call
/// `add_operation` (preserving PROTOCOL_VERSION_11 chain replay).
type DataTrigger = fn(
    &DocumentTransitionAction,
    &mut DataTriggerExecutionContext<'_>,
    &PlatformVersion,
) -> Result<DataTriggerExecutionResult, Error>;

/// A type alias for a [SimpleValidationResult] with a [DataTriggerError] as the error type.
///
/// This type is used to represent the result of executing a data trigger on the blockchain. It contains either a
/// successful result or a `DataTriggerActionError`, indicating the failure of the trigger.
pub(super) type DataTriggerExecutionResult = SimpleValidationResult<DataTriggerError>;
