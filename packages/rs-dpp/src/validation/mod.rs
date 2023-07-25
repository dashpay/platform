#[cfg(feature = "validation")]
use async_trait::async_trait;
#[cfg(feature = "validation")]
pub use json_schema_validator::JsonSchemaValidator;
#[cfg(feature = "validation")]
#[cfg(test)]
use mockall::automock;
#[cfg(feature = "validation")]
#[cfg(test)]
use serde_json::Value as JsonValue;
pub use validation_result::{
    ConsensusValidationResult, SimpleConsensusValidationResult, SimpleValidationResult,
    ValidationResult,
};

use crate::version::PlatformVersion;
#[cfg(feature = "validation")]
use crate::ProtocolError;

#[cfg(feature = "validation")]
pub mod block_time_window;
#[cfg(feature = "validation")]
pub mod byte_array_meta;
#[cfg(feature = "validation")]
pub mod json_schema_validator;
#[cfg(feature = "validation")]
mod meta_validators;
mod validation_result;

#[cfg(feature = "validation")]
/// Validator validates data of given type
pub trait DataValidator {
    // TODO, when GAT is available remove the reference in method and use: `type Item<'a>`
    type Item;
    fn validate(
        &self,
        data: &Self::Item,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}
//
// #[cfg(feature = "validation")]
// /// Async validator validates data of given type
// #[async_trait(?Send)]
// pub trait AsyncDataValidator {
//     type Item;
//     type ResultItem: Clone;
//     async fn validate(
//         &self,
//         data: &Self::Item,
//         execution_context: &StateTransitionExecutionContext,
//     ) -> Result<ConsensusValidationResult<Self::ResultItem>, ProtocolError>;
// }
//
// #[cfg(feature = "validation")]
// /// Validator takes additionally an execution context and generates fee
// pub trait DataValidatorWithContext {
//     // TODO, when GAT is available remove the reference in method and use: `type Item<'a>`
//     type Item;
//     fn validate(
//         &self,
//         data: &Self::Item,
//         execution_context: &StateTransitionExecutionContext,
//     ) -> Result<SimpleConsensusValidationResult, ProtocolError>;
// }
//
// #[cfg(feature = "validation")]
// /// Async validator takes additionally an execution context and generates fee
// #[cfg_attr(test, automock(type Item = JsonValue;))]
// #[async_trait(?Send)]
// pub trait AsyncDataValidatorWithContext {
//     // TODO, when GAT is available remove the reference in method and use: `type Item<'a>`
//     type Item;
//     async fn validate(
//         &self,
//         data: &Self::Item,
//         execution_context: &StateTransitionExecutionContext,
//     ) -> Result<SimpleConsensusValidationResult, ProtocolError>;
// }
