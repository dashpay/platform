#[cfg(feature = "validation-schema")]
use crate::data_contract::document_type::schema::validation_schema::DocumentSchemaValidator;
#[cfg(feature = "validation-schema")]
use crate::data_contract::errors::DataContractError;
use crate::validation::JsonSchemaValidator;
#[cfg(feature = "validation-schema")]
use platform_version::version::FeatureVersion;
use std::fmt;
use std::ops::Deref;
#[cfg(feature = "validation-schema")]
use std::sync::OnceLock;

/// DocumentType requires all fields to implement PartialEq and Clone
/// JsonSchemaLazyValidator builds on demand with schema from DocumentType
/// so there is no need to compare or clone it. We just reset it and it will
/// re-build on the next validation
///
/// Beside the `jsonschema` validator it holds the native one, built on demand the same
/// way, or the refusal of a document type's schema that has no validation schema, whose
/// documents protocol version 15 refuses: one slot for each generation of the validation
/// schema's builder (`validation_schema`).
pub struct StatelessJsonSchemaLazyValidator(
    JsonSchemaValidator,
    #[cfg(feature = "validation-schema")] [NativeValidatorSlot; 1],
);

/// What document validation builds of a document type's schema with one generation of the
/// validation schema's builder: the native validator, or the refusal of the schema.
#[cfg(feature = "validation-schema")]
pub(crate) type NativeValidatorSlot = OnceLock<Result<DocumentSchemaValidator, DataContractError>>;

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
            [OnceLock::new()],
        )
    }

    /// The slot of builder generation `generation`, `None` for a generation without one.
    /// Each generation keeps what it builds apart, so a protocol version selecting another
    /// generation never reads a validator the first built.
    #[cfg(feature = "validation-schema")]
    pub(crate) fn native(&self, generation: FeatureVersion) -> Option<&NativeValidatorSlot> {
        self.1.get(usize::from(generation))
    }
}

/// Prints the `jsonschema` validator alone, as before the native one was added: the native
/// validator is a cache of what the document type's schema builds, which `PartialEq` and
/// `Clone` leave out too, so a document type prints the same whether or not it is built.
impl fmt::Debug for StatelessJsonSchemaLazyValidator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("StatelessJsonSchemaLazyValidator")
            .field(&self.0)
            .finish()
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
