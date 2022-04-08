use std::path::{Display, Path};
use jsonschema::error::ValidationErrorKind;
use jsonschema::paths::{JSONPointer, PathChunk};
use jsonschema::ValidationError;
use thiserror::Error;
use crate::errors::consensus::{AbstractConsensusError, ConsensusError};

#[derive(Error, Debug)]
#[error("JsonSchemaError: {message:?}, kind: {kind:?}, instance_path: {instance_path:?}, schema_path:{schema_path:?}")]
pub struct JsonSchemaError {
    message: String,
    // TODO: deconstruct this - keyword is no a kind, but kind contains some additional data
    kind: ValidationErrorKind,
    instance_path: JSONPointer,
    schema_path: JSONPointer,
    // This is stored inside the keyword (error kind)
    // this.params = params;
    // this.propertyName = propertyName;
}

// fn weired_clone(kind: &ValidationErrorKind) -> ValidationErrorKind {
//     match kind {
//         ValidationErrorKind::AdditionalItems { limit } => { ValidationErrorKind::AdditionalItems { limit: *limit } }
//         ValidationErrorKind::AdditionalProperties { .. } => {ValidationErrorKind::AdditionalProperties}
//         ValidationErrorKind::AnyOf => {ValidationErrorKind::AnyOf}
//         ValidationErrorKind::BacktrackLimitExceeded { .. } => {ValidationErrorKind::BacktrackLimitExceeded}
//         ValidationErrorKind::Constant { .. } => {ValidationErrorKind::Constant}
//         ValidationErrorKind::Contains => {ValidationErrorKind::Contains}
//         ValidationErrorKind::ContentEncoding { .. } => {ValidationErrorKind::ContentEncoding}
//         ValidationErrorKind::ContentMediaType { .. } => {ValidationErrorKind::ContentMediaType}
//         ValidationErrorKind::Enum { .. } => {ValidationErrorKind::Enum}
//         ValidationErrorKind::ExclusiveMaximum { .. } => {ValidationErrorKind::ExclusiveMaximum}
//         ValidationErrorKind::ExclusiveMinimum { .. } => {ValidationErrorKind::ExclusiveMinimum}
//         ValidationErrorKind::FalseSchema => {ValidationErrorKind::FalseSchema}
//         ValidationErrorKind::FileNotFound { .. } => {ValidationErrorKind::FileNotFound}
//         ValidationErrorKind::Format { .. } => {ValidationErrorKind::Format}
//         ValidationErrorKind::FromUtf8 { .. } => {ValidationErrorKind::FromUtf8}
//         ValidationErrorKind::Utf8 { .. } => {ValidationErrorKind::Utf8}
//         ValidationErrorKind::JSONParse { .. } => {ValidationErrorKind::JSONParse}
//         ValidationErrorKind::InvalidReference { .. } => {ValidationErrorKind::InvalidReference}
//         ValidationErrorKind::InvalidURL { .. } => {ValidationErrorKind::InvalidURL}
//         ValidationErrorKind::MaxItems { .. } => {ValidationErrorKind::MaxItems}
//         ValidationErrorKind::Maximum { .. } => {ValidationErrorKind::Maximum}
//         ValidationErrorKind::MaxLength { .. } => {ValidationErrorKind::MaxLength}
//         ValidationErrorKind::MaxProperties { .. } => {ValidationErrorKind::MaxProperties}
//         ValidationErrorKind::MinItems { .. } => {ValidationErrorKind::MinItems}
//         ValidationErrorKind::Minimum { .. } => {ValidationErrorKind::Minimum}
//         ValidationErrorKind::MinLength { .. } => {ValidationErrorKind::MinLength}
//         ValidationErrorKind::MinProperties { .. } => {ValidationErrorKind::MinProperties}
//         ValidationErrorKind::MultipleOf { .. } => {ValidationErrorKind::MultipleOf}
//         ValidationErrorKind::Not { .. } => {ValidationErrorKind::Not}
//         ValidationErrorKind::OneOfMultipleValid => { ValidationErrorKind::OneOfMultipleValid }
//         ValidationErrorKind::OneOfNotValid => { ValidationErrorKind::OneOfNotValid }
//         ValidationErrorKind::Pattern { .. } => { ValidationErrorKind::Pattern }
//         ValidationErrorKind::PropertyNames { .. } => { ValidationErrorKind::PropertyNames }
//         ValidationErrorKind::Required { .. } => { ValidationErrorKind::Required }
//         ValidationErrorKind::Schema => { ValidationErrorKind::Schema }
//         ValidationErrorKind::Type { .. } => { ValidationErrorKind::Type }
//         ValidationErrorKind::UniqueItems => { ValidationErrorKind::UniqueItems }
//         ValidationErrorKind::UnknownReferenceScheme { .. } => { ValidationErrorKind::UnknownReferenceScheme }
//         ValidationErrorKind::Resolver { .. } => { ValidationErrorKind::Resolver }
//     }
// }

// impl Clone for JsonSchemaError {
//     fn clone(&self) -> Self {
//         Self {
//             message: self.message.clone(),
//             kind: self.kind.clone(),
//             instance_path: self.instance_path.clone(),
//             schema_path: self.schema_path.clone()
//         }
//     }
// }

impl AbstractConsensusError for JsonSchemaError {}

impl<'a> From<ValidationError<'a>> for JsonSchemaError {
    fn from(validation_error: ValidationError<'a>) -> Self {
        Self {
            // TODO: implement message
            message: String::new(),
            kind: validation_error.kind,
            instance_path: validation_error.instance_path,
            schema_path: validation_error.schema_path
        }
    }
}

impl JsonSchemaError {
    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn instance_path(&self) -> &JSONPointer {
        &self.instance_path
    }

    pub fn schema_path(&self) -> &JSONPointer {
        &self.schema_path
    }

    pub fn kind(&self) -> &ValidationErrorKind {
        &self.kind
    }

    pub fn keyword(&self) -> Option<&str> {
        let chunk = self.schema_path.last()?;
        match chunk {
            PathChunk::Property(_) => { None }
            PathChunk::Index(_) => { None }
            PathChunk::Keyword(keyword) => {Some(keyword)}
        }
    }

    // Kind was called "params" in the original reference
    pub fn params(&self) -> &ValidationErrorKind {
        self.kind()
    }
}