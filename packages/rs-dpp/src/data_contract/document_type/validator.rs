#[cfg(feature = "validation-schema")]
use crate::data_contract::document_type::schema::validation_schema::{
    DocumentSchemaValidator, ValidationSchema,
};
#[cfg(feature = "validation-schema")]
use crate::data_contract::errors::DataContractError;
use crate::validation::JsonSchemaValidator;
#[cfg(feature = "validation-schema")]
use platform_version::version::FeatureVersion;
use std::fmt;
use std::ops::Deref;
#[cfg(feature = "validation-schema")]
use std::sync::{Arc, OnceLock};

/// DocumentType requires all fields to implement PartialEq and Clone
/// JsonSchemaLazyValidator builds on demand with schema from DocumentType
/// so there is no need to compare or clone it. We just reset it and it will
/// re-build on the next validation
///
/// Beside the `jsonschema` validator it holds the native one, built on demand the same
/// way, or the refusal of a document type's schema that has no validation schema, whose
/// documents protocol version 15 refuses: one slot for each generation of the validation
/// schema's builder (`validation_schema`).
///
/// A document type read back from its stored, compiled form also holds what it was stored with
/// to check its documents ([`LoadedValidationSchema`]). That one is data, not a cache: `Clone`
/// keeps it, so its documents keep being checked as the version that stored it checks them.
/// `PartialEq` and `Debug` leave it out, as the stored form is compiled from the schema the
/// document type holds.
pub struct StatelessJsonSchemaLazyValidator(
    JsonSchemaValidator,
    #[cfg(feature = "validation-schema")] [NativeValidatorSlot; 1],
    #[cfg(feature = "validation-schema")] Option<Arc<LoadedValidationSchema>>,
);

/// What a document type read back from its stored form was stored with to check its
/// documents: the typed validation schema and the generation of the validation schema's
/// builder that built it, or, for a stored form holding none (the builder of the version that
/// stored it refused its schema, or that version selected no builder), the refusal of its
/// documents. Either way they are checked as that version checks them, at every later version:
/// a later builder that would accept the schema does not make them valid.
#[cfg(feature = "validation-schema")]
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum LoadedValidationSchema {
    Built {
        generation: FeatureVersion,
        schema: ValidationSchema,
    },
    Refused(DataContractError),
}

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
            #[cfg(feature = "validation-schema")]
            None,
        )
    }

    /// The validator of a document type read back from its stored form with `loaded`, what it
    /// was stored with to check its documents.
    #[cfg(feature = "validation-schema")]
    pub(crate) fn with_loaded_validation_schema(loaded: LoadedValidationSchema) -> Self {
        Self(
            JsonSchemaValidator::new(),
            [OnceLock::new()],
            Some(Arc::new(loaded)),
        )
    }

    /// What the document type was read back with from its stored form to check its documents,
    /// `None` for a document type parsed from its schema.
    #[cfg(feature = "validation-schema")]
    pub(crate) fn loaded_validation_schema(&self) -> Option<&LoadedValidationSchema> {
        self.2.as_deref()
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

/// Leaves the caches empty and keeps the stored typed validation schema, which is data.
impl Clone for StatelessJsonSchemaLazyValidator {
    fn clone(&self) -> Self {
        Self(
            JsonSchemaValidator::new(),
            #[cfg(feature = "validation-schema")]
            [OnceLock::new()],
            #[cfg(feature = "validation-schema")]
            self.2.clone(),
        )
    }

    fn clone_from(&mut self, source: &Self) {
        *self = source.clone();
    }
}
