//! The native validator of a document type's schema, [`DocumentSchemaValidator`], built and run
//! by the generation of `validation_schema` that built it. Generation 0, which protocol version
//! 15 selects, is in `v0`.

use super::ValidationSchema;
use crate::consensus::basic::json_schema_error::JsonSchemaError;
use crate::data_contract::errors::DataContractError;
use crate::ProtocolError;
use platform_version::version::FeatureVersion;
use regex::Regex;
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;

mod v0;

/// A document type's validation schema ready to check document values: its patterns
/// compiled once, and the generation of `validation_schema` that built it, which checks them.
#[derive(Debug)]
pub struct DocumentSchemaValidator {
    generation: FeatureVersion,
    schema: ValidationSchema,
    patterns: BTreeMap<String, Regex>,
}

impl DocumentSchemaValidator {
    /// The validator of `schema` as generation 0 builds it. Refuses a `pattern` the JSON
    /// Schema validator does not compile and a property declared twice, which the builder
    /// refuses already, and a chain of `$ref`s that returns to where it started without
    /// entering the value.
    pub fn new(schema: ValidationSchema) -> Result<Self, DataContractError> {
        Self::with_patterns_v0(schema, BTreeMap::new())
    }

    /// The validator of `schema` as generation `generation` of `validation_schema` builds it,
    /// reusing `patterns`, the regexes the builder compiled. Refuses a generation this module
    /// does not know.
    pub(crate) fn with_patterns(
        schema: ValidationSchema,
        patterns: BTreeMap<String, Regex>,
        generation: FeatureVersion,
    ) -> Result<Self, ProtocolError> {
        match generation {
            0 => Ok(Self::with_patterns_v0(schema, patterns)?),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentSchemaValidator::with_patterns".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    /// The first violation of `instance`, the JSON the document validator converts a
    /// document's properties to, as the JSON Schema error that reports it and as the
    /// generation that built the validator checks it: none when it satisfies the schema.
    /// Consensus reports only the first violation, so the check stops there.
    pub fn validate(&self, instance: &JsonValue) -> Result<Option<JsonSchemaError>, ProtocolError> {
        match self.generation {
            0 => self.validate_v0(instance),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentSchemaValidator::validate".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
