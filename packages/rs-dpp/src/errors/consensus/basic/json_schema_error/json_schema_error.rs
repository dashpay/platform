use crate::consensus::basic::json_schema_error::json_schema_error_data::JsonSchemaErrorData;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use jsonschema::ValidationError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Value;
use serde_json::Value as JsonValue;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("JsonSchemaError: {error_summary}, path: {instance_path}")]
#[platform_serialize(unversioned)]
pub struct JsonSchemaError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    error_summary: String,
    keyword: String,
    instance_path: String,
    schema_path: String,
    params: Value,
    property_name: String,
}

impl<'a> From<ValidationError<'a>> for JsonSchemaError {
    fn from(validation_error: ValidationError<'a>) -> Self {
        let JsonSchemaErrorData {
            keyword,
            params,
            property_name,
            error_message,
        } = JsonSchemaErrorData::from(&validation_error);

        Self {
            keyword,
            error_summary: error_message,
            instance_path: validation_error.instance_path.to_string(),
            schema_path: validation_error.schema_path.to_string(),
            params: JsonValue::Object(params).into(),
            property_name,
        }
    }
}

impl<'a> From<ValidationError<'a>> for ConsensusError {
    fn from(validation_error: ValidationError<'a>) -> Self {
        Self::BasicError(BasicError::JsonSchemaError(JsonSchemaError::from(
            validation_error,
        )))
    }
}

impl From<JsonSchemaError> for ConsensusError {
    fn from(e: JsonSchemaError) -> Self {
        Self::BasicError(BasicError::JsonSchemaError(e))
    }
}

impl JsonSchemaError {
    pub fn error_summary(&self) -> &str {
        &self.error_summary
    }

    pub fn keyword(&self) -> &str {
        &self.keyword
    }

    pub fn instance_path(&self) -> &str {
        &self.instance_path
    }

    pub fn schema_path(&self) -> &str {
        &self.schema_path
    }

    pub fn property_name(&self) -> &str {
        &self.property_name
    }

    pub fn params(&self) -> &Value {
        &self.params
    }
}
