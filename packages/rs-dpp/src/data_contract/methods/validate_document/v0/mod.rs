use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::DocumentType;

use crate::consensus::basic::document::{
    DocumentFieldMaxSizeExceededError, InvalidDocumentTypeError,
};
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::schema::DataContractSchemaMethodsV0;
use crate::data_contract::DataContract;
use crate::document::{Document, DocumentV0Getters};
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_value::Value;
use platform_version::version::PlatformVersion;
use std::ops::Deref;

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

        let validator = document_type.json_schema_validator_ref().deref();

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

    #[inline(always)]
    pub(super) fn validate_document_v0(
        &self,
        name: &str,
        document: &Document,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        // Validate user defined properties
        self.validate_document_properties_v0(name, document.properties().into(), platform_version)
    }
}
