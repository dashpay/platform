
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::schema::enrich_with_base_schema;
use crate::data_contract::document_type::{DocumentTypeRef};

use crate::data_contract::schema::DataContractSchemaMethodsV0;
use crate::data_contract::DataContract;
use crate::document::serialization_traits::DocumentPlatformValueMethodsV0;
use crate::document::Document;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_value::Value;
use platform_version::version::PlatformVersion;
use std::ops::Deref;

pub trait DataContractValidationMethodsV0 {
    fn validate_document(
        &mut self,
        name: &str,
        document: &Document,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>;

    fn validate_document_value(
        &mut self,
        name: &str,
        value: &Value,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

impl DataContract {
    pub(super) fn validate_document_value_v0(
        &mut self,
        name: &str,
        value: &Value,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let document_type = self.document_type_for_name(name)?;

        let validator = match document_type {
            DocumentTypeRef::V0(v0) => v0.json_schema_validator.deref(),
        };

        // Compile json schema validator if it's not yet compiled
        if !validator.is_compiled(platform_version)? {
            let root_schema = enrich_with_base_schema(
                document_type.schema().clone(),
                self.schema_defs().map(|defs| Value::from(defs.clone())),
                &[],
                platform_version,
            )?;

            let root_json_schema = root_schema
                .try_to_validating_json()
                .map_err(ProtocolError::ValueError)?;

            validator.compile(&root_json_schema, platform_version)?;
        }

        let json_value = value
            .try_to_validating_json()
            .map_err(ProtocolError::ValueError)?;

        validator.validate(&json_value, platform_version)
    }

    pub(super) fn validate_document_v0(
        &mut self,
        name: &str,
        document: &Document,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let value = document.to_object()?;

        self.validate_document_value_v0(name, &value, platform_version)
    }
}
