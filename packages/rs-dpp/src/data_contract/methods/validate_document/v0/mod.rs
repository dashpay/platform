use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::DocumentType;

use crate::data_contract::document_type::v0::StatelessJsonSchemaLazyValidator;
use crate::data_contract::schema::DataContractSchemaMethodsV0;
use crate::data_contract::DataContract;
use crate::document::{property_names, Document, DocumentV0Getters};
use crate::errors::consensus::basic::document::{
    DocumentFieldMaxSizeExceededError, InvalidDocumentTypeError, MissingDocumentTypeError,
};
use crate::errors::consensus::basic::BasicError;
use crate::errors::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::validation::SimpleConsensusValidationResult;
use platform_value::Value;
use platform_version::version::PlatformVersion;

pub trait DataContractDocumentValidationMethodsV0 {
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

        let validator = StatelessJsonSchemaLazyValidator::new();
        // let validator = match document_type {
        //     DocumentTypeRef::V0(v0) => v0.json_schema_validator.deref(),
        // };

        if let Some((key, size)) =
            value.has_data_larger_than(platform_version.system_limits.max_field_value_size)
        {
            let field = match key {
                Some(Value::Text(field)) => field,
                _ => "".to_string(),
            };
            return Ok(SimpleConsensusValidationResult::new_with_error(
                ConsensusError::BasicError(BasicError::DocumentFieldMaxSizeExceededError(
                    DocumentFieldMaxSizeExceededError::new(
                        field,
                        size as u64,
                        platform_version.system_limits.max_field_value_size as u64,
                    ),
                )),
            ));
        }

        let json_value = match value.try_into_validating_json() {
            Ok(json_value) => json_value,
            Err(e) => {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::BasicError(BasicError::ValueError(e.into())),
                ))
            }
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

            validator.compile_and_validate(&root_json_schema, &json_value, platform_version)
        } else {
            validator.validate(&json_value, platform_version)
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
