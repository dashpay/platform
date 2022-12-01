use async_trait::async_trait;
pub use json_schema_validator::JsonSchemaValidator;
pub use validation_result::{SimpleValidationResult, ValidationResult};

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
    fn validate(&self, data: &Self::Item) -> Result<SimpleValidationResult, ProtocolError>;
}

/// Async validator validates data of given type
#[async_trait]
pub trait AsyncDataValidator {
    // TODO, when GAT is available remove the reference in method and use: `type Item<'a>`
    type Item;
    async fn validate(&self, data: &Self::Item) -> Result<SimpleValidationResult, ProtocolError>;
}

/// Validator takes additionally an execution context and generates fee
pub trait DataValidatorWithContext {
    // TODO, when GAT is available remove the reference in method and use: `type Item<'a>`
    type Item;
    fn validate(
        &self,
        data: &Self::Item,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<SimpleValidationResult, ProtocolError>;
}

/// Async validator takes additionally an execution context and generates fee
#[async_trait]
pub trait AsyncDataValidatorWithContext {
    // TODO, when GAT is available remove the reference in method and use: `type Item<'a>`
    type Item;
    async fn validate(&self, data: &Self::Item) -> Result<SimpleValidationResult, ProtocolError>;
}
