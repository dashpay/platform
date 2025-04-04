use crate::validation::JsonSchemaValidator;
use std::ops::Deref;

/// DocumentType requires all fields to implement PartialEq and Clone
/// JsonSchemaLazyValidator builds on demand with schema from DocumentType
/// so there is no need to compare or clone it. We just reset it and it will
/// re-build on the next validation

#[derive(Debug)]
pub struct StatelessJsonSchemaLazyValidator(JsonSchemaValidator);

impl Default for StatelessJsonSchemaLazyValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl StatelessJsonSchemaLazyValidator {
    pub fn new() -> Self {
        Self(JsonSchemaValidator::new())
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
        self.0 = JsonSchemaValidator::new();
    }
}
