use async_trait::async_trait;

#[cfg(test)]
use mockall::automock;

#[cfg(test)]
use serde_json::Value as JsonValue;

pub use json_schema_validator::JsonSchemaValidator;
pub use validation_result::{
    ConsensusValidationResult, SimpleConsensusValidationResult, SimpleValidationResult,
    ValidationResult,
};

use crate::{
    state_transition::state_transition_execution_context::StateTransitionExecutionContext,
    ProtocolError,
};

pub mod byte_array_meta;
mod json_schema_validator;
mod meta_validators;
mod validation_result;

/// Validator validates data of given type
pub trait DataValidator {
    // TODO, when GAT is available remove the reference in method and use: `type Item<'a>`
    type Item;
    fn validate(&self, data: &Self::Item)
        -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

/// Async validator validates data of given type
#[async_trait(?Send)]
pub trait AsyncDataValidator {
    type Item;
    type ResultItem: Clone;
    async fn validate(
        &self,
        data: &Self::Item,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<ConsensusValidationResult<Self::ResultItem>, ProtocolError>;
}

/// Validator takes additionally an execution context and generates fee
pub trait DataValidatorWithContext {
    // TODO, when GAT is available remove the reference in method and use: `type Item<'a>`
    type Item;
    fn validate(
        &self,
        data: &Self::Item,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

/// Async validator takes additionally an execution context and generates fee
#[cfg_attr(test, automock(type Item = JsonValue;))]
#[async_trait(?Send)]
pub trait AsyncDataValidatorWithContext {
    // TODO, when GAT is available remove the reference in method and use: `type Item<'a>`
    type Item;
    async fn validate(
        &self,
        data: &Self::Item,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}
