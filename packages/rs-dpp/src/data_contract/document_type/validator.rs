#[cfg(feature = "validation-schema")]
use crate::data_contract::document_type::schema::validation_schema::DocumentSchemaValidator;
use crate::validation::JsonSchemaValidator;
use std::ops::Deref;
#[cfg(feature = "validation-schema")]
use std::sync::{Arc, OnceLock};

/// DocumentType requires all fields to implement PartialEq and Clone
/// JsonSchemaLazyValidator builds on demand with schema from DocumentType
/// so there is no need to compare or clone it. We just reset it and it will
/// re-build on the next validation
///
/// Beside the `jsonschema` validator it holds the native one, built on demand the same
/// way: `None` once built when the document type's schema has no validation schema, whose
/// documents the `jsonschema` validator then checks.
#[derive(Debug)]
pub struct StatelessJsonSchemaLazyValidator(
    JsonSchemaValidator,
    #[cfg(feature = "validation-schema")] OnceLock<Option<Arc<DocumentSchemaValidator>>>,
);

impl Default for StatelessJsonSchemaLazyValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl StatelessJsonSchemaLazyValidator {
    pub fn new() -> Self {
        Self(
            JsonSchemaValidator::new(),
            #[cfg(feature = "validation-schema")]
            OnceLock::new(),
        )
    }

    /// The native validator, once built.
    #[cfg(feature = "validation-schema")]
    pub(crate) fn native(&self) -> &OnceLock<Option<Arc<DocumentSchemaValidator>>> {
        &self.1
    }
}

impl Deref for StatelessJsonSchemaLazyValidator {
    type Target = JsonSchemaValidator;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PartialEq for StatelessJsonSchemaLazyValidator {
    // We assume that validator is stateless and initialized by the schema from DocumentType
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Clone for StatelessJsonSchemaLazyValidator {
    fn clone(&self) -> Self {
        StatelessJsonSchemaLazyValidator::new()
    }

    fn clone_from(&mut self, _source: &Self) {
        *self = StatelessJsonSchemaLazyValidator::new();
    }
}
