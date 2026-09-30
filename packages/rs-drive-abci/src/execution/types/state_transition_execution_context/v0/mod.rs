use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::signing_key_limits::SigningKeyLimits;

/// The V0 struct of the state transition execution context
#[derive(Debug, Clone, Default)]
pub struct StateTransitionExecutionContextV0 {
    // Are we executing the state transition in a dry run
    // Dry run is execution on check tx
    /// Are we in a dry run?
    pub dry_run: bool,
    /// The execution operations
    pub operations: Vec<ValidationOperation>,
    /// The usage limits of the key that signed the state transition, when it has any. Set by
    /// identity signature validation from protocol version 14.
    pub signing_key_limits: Option<SigningKeyLimits>,
}
