use std::borrow::Cow;
use std::path::Display;
use jsonschema::error::ValidationErrorKind;
use jsonschema::paths::JSONPointer;
use jsonschema::ValidationError;
use thiserror::Error;

fn into_owned(err: ValidationError) -> ValidationError<'static> {
    ValidationError {
        instance_path: err.instance_path.clone(),
        instance: Cow::Owned(err.instance.into_owned()),
        kind: err.kind,
        schema_path: err.schema_path,
    }
}

#[derive(Error, Debug)]
#[error("Please implement me")]
pub struct JsonSchemaError {
    message: String,
    // TODO: deconstruct this - keyword is no a kind, but kind contains some additional data
    keyword: ValidationErrorKind,
    instance_path: JSONPointer,
    schema_path: JSONPointer,
    // This is stored inside the keyword (error kind)
    // this.params = params;
    // this.propertyName = propertyName;
}

impl<'a> From<ValidationError<'a>> for JsonSchemaError {
    fn from(validation_error: ValidationError<'a>) -> Self {
        Self {
            // TODO: implement message
            message: String::new(),
            keyword: validation_error.kind,
            instance_path: validation_error.instance_path,
            schema_path: validation_error.schema_path
        }
    }
}