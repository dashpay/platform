use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::platform_types::platform::PlatformStateRef;
use dpp::prelude::*;
use drive::grovedb::TransactionArg;

/// DataTriggerExecutionContext represents the context in which a data trigger is executed.
/// It contains references to relevant state and transaction data needed for the trigger to perform its actions.
#[derive(Debug, Clone)]
pub struct DataTriggerExecutionContext<'a> {
    /// A reference to the platform state, which contains information about the current blockchain environment.
    pub platform: &'a PlatformStateRef<'a>,
    /// The transaction argument that triggered the data trigger.
    pub transaction: TransactionArg<'a, 'a>,
    /// The identifier of the owner of the data contract that the trigger is associated with.
    pub owner_id: &'a Identifier,
    /// A reference to the execution context for the state transition that triggered the data trigger.
    pub state_transition_execution_context: &'a StateTransitionExecutionContext,
}
