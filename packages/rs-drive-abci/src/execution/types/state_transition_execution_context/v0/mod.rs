use crate::execution::types::execution_operation::ExecutionOperation;

#[derive(Debug, Clone, Default)]
pub struct StateTransitionExecutionContextV0 {
    // Are we executing the state transition in a dry run
    // Dry run is execution on check tx
    pub dry_run: bool,
    pub operations: Vec<ExecutionOperation>,
}
