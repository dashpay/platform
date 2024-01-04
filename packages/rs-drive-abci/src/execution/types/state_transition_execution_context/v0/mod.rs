use crate::execution::types::execution_operation::ExecutionOperation;

/// The V0 struct of the state transition execution context
#[derive(Debug, Clone, Default)]
pub struct StateTransitionExecutionContextV0 {
    // Are we executing the state transition in a dry run
    // Dry run is execution on check tx
    /// Are we in a dry run?
    pub dry_run: bool,
    /// The execution operations
    pub operations: Vec<ExecutionOperation>,
}
