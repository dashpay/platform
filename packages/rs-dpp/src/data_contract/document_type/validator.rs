#[cfg(feature = "validation-schema")]
use crate::data_contract::document_type::schema::validation_schema::DocumentSchemaValidator;
#[cfg(feature = "validation-schema")]
use crate::data_contract::document_type::serialized_version::v0::EncodedValidationSchema;
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
/// schema's builder (`validation_schema`). Registration fills the slot when it checks the
/// schema, so the same build serves document validation and the type's stored form.
///
/// A document type read back from its stored, compiled form also holds what it was stored with
/// to check its documents ([`LoadedValidationSchema`]). That one is data, not a cache: `Clone`
/// keeps it, so its documents keep being checked as the version that stored it checks them.
/// `PartialEq` and `Debug` leave it out: equality compares what a document type declares, not
/// how its documents are checked. A type read back from state equals the type parsed from the
/// same schema even where the two check documents differently, the read-back one with the
/// builder generation of the version that stored it, or refusing its documents where that
/// version built no validation schema. Drive relies on it: rebuilding a system contract's
/// document types (`Drive::apply_contract_rebuilding_document_types`) leaves a type whose
/// stored form equals the new one as it is, so running it again over its own result deletes
/// nothing; if a type read back from compiled storage never equaled a parsed one, every run
/// would delete its documents again. To compare how two types check their documents, compare
/// their `loaded_validation_schema()` as well.
pub struct StatelessJsonSchemaLazyValidator(
    JsonSchemaValidator,
    #[cfg(feature = "validation-schema")] [NativeValidatorSlot; 1],
    #[cfg(feature = "validation-schema")] Option<Arc<LoadedValidationSchema>>,
);

/// What a document type read back from its stored form was stored with to check its
/// documents: the typed validation schema, as stored, and the generation of the validation
/// schema's builder that built it; or the refusal of its documents, when the builder of the
/// version that stored it refused its schema, or that version selected no builder. Either way
/// they are checked as that version checks them, at every later version: a later builder that
/// would accept the schema does not make them valid.
#[cfg(feature = "validation-schema")]
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum LoadedValidationSchema {
    /// Built by builder `generation`; `schema` is its encoding, decoded on first use.
    Built {
        generation: FeatureVersion,
        schema: EncodedValidationSchema,
    },
    /// Refused by builder `generation`, or, with none, because the version that stored the
    /// type selected no builder.
    Refused {
        generation: Option<FeatureVersion>,
        refusal: DataContractError,
    },
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
/// validator is a cache of what the document type's schema builds, which `PartialEq` leaves
/// out too, so a document type prints the same whether or not it is built.
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

/// Always equal: the validators are caches built from the document type's schema, and the
/// stored validation schema is left out so equality compares what a document type declares
/// (see [`StatelessJsonSchemaLazyValidator`]).
impl PartialEq for StatelessJsonSchemaLazyValidator {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

/// Keeps the native validators built so far, which share their validation schema, and the
/// stored typed validation schema, which is data; resets the `jsonschema` validator, which
/// rebuilds on first use. A clone holds the same schema, so what was built of it holds for the
/// clone too; a document type whose schema changes in place drops what was built
/// (`DocumentTypeV0MutGetters::schema_mut`).
impl Clone for StatelessJsonSchemaLazyValidator {
    fn clone(&self) -> Self {
        Self(
            JsonSchemaValidator::new(),
            #[cfg(feature = "validation-schema")]
            self.1.clone(),
            #[cfg(feature = "validation-schema")]
            self.2.clone(),
        )
    }

    fn clone_from(&mut self, source: &Self) {
        *self = source.clone();
    }
}
