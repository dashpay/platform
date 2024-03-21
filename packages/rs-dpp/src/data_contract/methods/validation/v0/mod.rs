use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::{DocumentType, DocumentTypeRef};

use crate::consensus::basic::document::{InvalidDocumentTypeError, MissingDocumentTypeError};
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::schema::DataContractSchemaMethodsV0;
use crate::data_contract::DataContract;
use crate::document::{property_names, Document, DocumentV0Getters};
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_value::Value;
use platform_version::version::PlatformVersion;
use std::ops::Deref;

pub trait DataContractValidationMethodsV0 {
    fn validate_document(
        &self,
        name: &str,
        document: &Document,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>;

    fn validate_document_properties(
        &self,
        name: &str,
        value: Value,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

impl DataContract {
    #[inline(always)]
    pub(super) fn validate_document_properties_v0(
        &self,
        name: &str,
        value: Value,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let Some(document_type) = self.document_type_optional_for_name(name) else {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDocumentTypeError::new(name.to_owned(), self.id()).into(),
            ));
        };

        let validator = match document_type {
            DocumentTypeRef::V0(v0) => v0.json_schema_validator.deref(),
        };

        // Compile json schema validator if it's not yet compiled
        if !validator.is_compiled(platform_version)? {
            // It is normal that we get a protocol error here, since the document type is coming
            // from the state
            let root_schema = DocumentType::enrich_with_base_schema(
                // TODO: I just wondering if we could you references here
                //  instead of cloning
                document_type.schema().clone(),
                self.schema_defs().map(|defs| Value::from(defs.clone())),
                platform_version,
            )?;

            let root_json_schema = root_schema
                .try_to_validating_json()
                .map_err(ProtocolError::ValueError)?;

            validator.compile(&root_json_schema, platform_version)?;
        }

        match value.try_into_validating_json() {
            Ok(json_value) => validator.validate(&json_value, platform_version),
            Err(e) => Ok(SimpleConsensusValidationResult::new_with_error(
                ConsensusError::BasicError(BasicError::ValueError(e.into())),
            )),
        }
    }

    // TODO: Move to document
    #[inline(always)]
    pub(super) fn validate_document_v0(
        &self,
        name: &str,
        document: &Document,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        // Make sure that the document type is defined in the contract
        let Some(document_type) = self.document_type_optional_for_name(name) else {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDocumentTypeError::new(name.to_string(), self.id()).into(),
            ));
        };

        // Make sure that timestamps are present if required
        let required_fields = document_type.required_fields();

        if required_fields.contains(property_names::CREATED_AT) && document.created_at().is_none() {
            // TODO: Create a special consensus error for this
            return Ok(SimpleConsensusValidationResult::new_with_error(
                MissingDocumentTypeError::new().into(),
            ));
        }

        if required_fields.contains(property_names::UPDATED_AT) && document.updated_at().is_none() {
            // TODO: Create a special consensus error for this
            return Ok(SimpleConsensusValidationResult::new_with_error(
                MissingDocumentTypeError::new().into(),
            ));
        }

        // Validate user defined properties
        self.validate_document_properties_v0(name, document.properties().into(), platform_version)
    }
}
