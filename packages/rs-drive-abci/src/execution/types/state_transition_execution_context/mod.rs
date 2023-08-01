use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_operation::ExecutionOperation;
use crate::execution::types::state_transition_execution_context::v0::StateTransitionExecutionContextV0;
use dpp::version::{DefaultForPlatformVersion, FeatureVersion, PlatformVersion};

pub mod v0;

#[derive(Debug, Clone)]
pub enum StateTransitionExecutionContext {
    V0(StateTransitionExecutionContextV0),
}

pub trait StateTransitionExecutionContextMethodsV0 {
    fn add_operation(&mut self, operation: ExecutionOperation);
    fn add_operations(&mut self, operations: Vec<ExecutionOperation>);
    fn operations_consume(self) -> Vec<ExecutionOperation>;
    fn operations_slice(&self) -> &[ExecutionOperation];
}

impl StateTransitionExecutionContextMethodsV0 for StateTransitionExecutionContext {
    fn add_operation(&mut self, operation: ExecutionOperation) {
        match self {
            StateTransitionExecutionContext::V0(v0) => v0.operations.push(operation),
        }
    }

    fn add_operations(&mut self, operations: Vec<ExecutionOperation>) {
        match self {
            StateTransitionExecutionContext::V0(v0) => v0.operations.extend(operations),
        }
    }

    fn operations_consume(self) -> Vec<ExecutionOperation> {
        match self {
            StateTransitionExecutionContext::V0(v0) => v0.operations,
        }
    }

    fn operations_slice(&self) -> &[ExecutionOperation] {
        match self {
            StateTransitionExecutionContext::V0(v0) => v0.operations.as_slice(),
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
