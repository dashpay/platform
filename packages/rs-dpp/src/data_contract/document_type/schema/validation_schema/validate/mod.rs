//! The native validator of a document type's schema, [`DocumentSchemaValidator`], built and run
//! by the generation of `validation_schema` that built it. Generation 0, which protocol version
//! 15 selects, is in `v0`.

use super::ValidationSchema;
use crate::consensus::ConsensusError;
use crate::data_contract::errors::DataContractError;
use crate::ProtocolError;
use platform_version::version::FeatureVersion;
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use std::sync::Arc;

mod pattern;
mod v0;

use pattern::PatternCell;

/// Registration's native schema checks compare values and read list uniqueness as
/// generation 0 does.
#[cfg(feature = "validation")]
pub(crate) use v0::{equal_json, is_unique};

/// A document type's validation schema ready to check document values: each of its patterns
/// checked to parse, and compiled the first time a value reaches it, and the generation of
/// `validation_schema` that built it, which checks them. The schema is shared, so a clone, or
/// a validator built from a schema read back from a stored form, copies no node of it.
#[derive(Debug, Clone)]
pub struct DocumentSchemaValidator {
    generation: FeatureVersion,
    schema: Arc<ValidationSchema>,
    /// Every `pattern` the schema holds, by the pattern as written.
    patterns: BTreeMap<String, PatternCell>,
}

impl DocumentSchemaValidator {
    /// The validator of `schema` as generation 0 builds it. Refuses a `pattern` that does
    /// not parse and a property declared twice, which the builder refuses already, and a
    /// chain of `$ref`s that returns to where it started without entering the value.
    pub fn new(schema: ValidationSchema) -> Result<Self, DataContractError> {
        Self::with_patterns_v0(Arc::new(schema))
    }

    /// The validator of `schema` as generation `generation` of `validation_schema` builds it,
    /// with every pattern it holds checked to parse and none compiled, whether the schema was
    /// built from a document schema or read back from a stored form. Refuses a generation this
    /// module does not know.
    pub(crate) fn with_patterns(
        schema: Arc<ValidationSchema>,
        generation: FeatureVersion,
    ) -> Result<Self, ProtocolError> {
        match generation {
            0 => Ok(Self::with_patterns_v0(schema)?),
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
    /// The generation of `validation_schema` that built the validator, and checks with it.
    pub(crate) fn generation(&self) -> FeatureVersion {
        self.generation
    }

    /// Compiles every pattern the validator holds, as registration does before it accepts a
    /// type, and refuses the first that does not compile: a value reaching it would be
    /// refused. The patterns stay compiled for the documents the validator checks.
    #[cfg(feature = "validation")]
    pub(crate) fn compile_patterns(&self) -> Result<(), ProtocolError> {
        match self.generation {
            0 => Ok(self.compile_patterns_v0()?),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentSchemaValidator::compile_patterns".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    /// How many of the validator's patterns it has compiled, or found not to compile.
    #[cfg(test)]
    pub(crate) fn attempted_patterns(&self) -> usize {
        self.patterns
            .values()
            .filter(|pattern| pattern.attempted())
            .count()
    }

    /// The validation schema the validator checks against.
    pub(crate) fn schema(&self) -> &ValidationSchema {
        &self.schema
    }

    /// The first violation of `instance` as the consensus error reporting it. The walk's
    /// recursion is bounded at `max_value_depth` value-nesting levels (`None` for no bound):
    /// the walk refuses a value it descends past that depth, with the error the document
    /// validator's depth check gives a too-deep value. That check, which runs before the
    /// walk, is what bounds a document's own value depth; this bound guards the walk's
    /// recursion for a caller that reaches it another way. The generation that built the
    /// validator checks it.
    pub fn validate(
        &self,
        instance: &JsonValue,
        max_value_depth: Option<usize>,
    ) -> Result<Option<ConsensusError>, ProtocolError> {
        match self.generation {
            0 => self.validate_v0(instance, max_value_depth),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentSchemaValidator::validate".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
