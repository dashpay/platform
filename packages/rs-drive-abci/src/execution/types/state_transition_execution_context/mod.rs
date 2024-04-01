use crate::error::execution::ExecutionError;

use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::v0::StateTransitionExecutionContextV0;
use derive_more::From;
use dpp::fee::fee_result::FeeResult;
use dpp::version::{DefaultForPlatformVersion, PlatformVersion};

/// V0 module
pub mod v0;

#[derive(Debug, Clone, From)]
/// The state transition execution context
pub enum StateTransitionExecutionContext {
    /// Version 0
    V0(StateTransitionExecutionContextV0),
}

/// The trait defining state transition execution context methods for v0
pub trait StateTransitionExecutionContextMethodsV0 {
    /// Add an operation to the state transition execution context
    fn add_operation(&mut self, operation: ValidationOperation);
    /// Add a operations to the state transition execution context
    fn add_operations(&mut self, operations: Vec<ValidationOperation>);
    /// Consume the operations of the context
    fn operations_consume(self) -> Vec<ValidationOperation>;
    /// Returns a slice of operations, does not consume the context
    fn operations_slice(&self) -> &[ValidationOperation];

    /// Are we in a dry run?
    fn in_dry_run(&self) -> bool;
    /// Set us to be in a dry run
    fn enable_dry_run(&mut self);
    /// Set us not to be in a dry run
    fn disable_dry_run(&mut self);

    /// Get the fee costs of all operations in the execution context
    fn fee_cost(&self, platform_version: &PlatformVersion) -> Result<FeeResult, Error>;
}

impl StateTransitionExecutionContextMethodsV0 for StateTransitionExecutionContext {
    fn add_operation(&mut self, operation: ValidationOperation) {
        match self {
            StateTransitionExecutionContext::V0(v0) => v0.operations.push(operation),
        }
    }

    fn add_operations(&mut self, operations: Vec<ValidationOperation>) {
        match self {
            StateTransitionExecutionContext::V0(v0) => v0.operations.extend(operations),
        }
    }

    fn operations_consume(self) -> Vec<ValidationOperation> {
        match self {
            StateTransitionExecutionContext::V0(v0) => v0.operations,
        }
    }

    fn operations_slice(&self) -> &[ValidationOperation] {
        match self {
            StateTransitionExecutionContext::V0(v0) => v0.operations.as_slice(),
        }
    }

    fn in_dry_run(&self) -> bool {
        match self {
            StateTransitionExecutionContext::V0(v0) => v0.dry_run,
        }
    }

    fn enable_dry_run(&mut self) {
        match self {
            StateTransitionExecutionContext::V0(v0) => v0.dry_run = true,
        }
    }

    fn disable_dry_run(&mut self) {
        match self {
            StateTransitionExecutionContext::V0(v0) => v0.dry_run = false,
        }
    }
    fn fee_cost(&self, platform_version: &PlatformVersion) -> Result<FeeResult, Error> {
        match self {
            StateTransitionExecutionContext::V0(v0) => {
                let mut fee_result = FeeResult::default();
                ValidationOperation::add_many_to_fee_result(
                    v0.operations.as_slice(),
                    &mut fee_result,
                    platform_version,
                )?;
                Ok(fee_result)
            }
        }
    }
}

impl DefaultForPlatformVersion for StateTransitionExecutionContext {
    type Error = ExecutionError;

    fn default_for_platform_version(
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        match platform_version
            .drive_abci
            .structs
            .state_transition_execution_context
        {
            0 => Ok(StateTransitionExecutionContextV0::default().into()),
            version => Err(ExecutionError::UnknownVersionMismatch {
                method: "DataContract::from_json_object".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
