#[cfg(feature = "validation")]
pub(crate) use json_schema_validator::JsonSchemaValidator;

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
pub(crate) mod byte_array_meta;
#[cfg(feature = "validation")]
mod json_schema_validator;
#[cfg(feature = "validation")]
pub(crate) mod meta_validators;
mod validation_result;

#[cfg(feature = "validation")]
/// Validator validates data of given type
pub trait DataValidator {
    // TODO We should remove it
    // TODO, when GAT is available remove the reference in method and use: `type Item<'a>`
    type Item;
    fn validate(
        &self,
        data: &Self::Item,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}
