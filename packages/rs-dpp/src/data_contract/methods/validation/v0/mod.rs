use crate::consensus::ConsensusError;
use crate::data_contract::document_type::schema::{create_validator, enrich_with_base_schema};
use crate::data_contract::document_type::DocumentType;
use crate::data_contract::errors::DataContractError;
use crate::data_contract::schema::DataContractSchemaMethodsV0;
use crate::data_contract::DataContract;
use crate::document::serialization_traits::DocumentPlatformValueMethodsV0;
use crate::document::Document;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_value::Value;
use platform_version::version::PlatformVersion;

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
        let mut document_type = match self {
            DataContract::V0(v0) => v0.document_types.get_mut(name).ok_or_else(|| {
                ProtocolError::DataContractError(DataContractError::DocumentTypeNotFound(name))
            })?,
        };

        let validator = match document_type {
            DocumentType::V0(v0) => {
                // Ensure we have a validator
                if v0.json_schema_validator.is_none() {
                    let json_schema = enrich_with_base_schema(
                        v0.schema.clone(),
                        self.schema_defs().map(|defs| Value::from(defs.clone())),
                        &[],
                        platform_version,
                    )?;

                    let json_schema_validator = create_validator(&json_schema, platform_version)?;

                    v0.json_schema_validator = Some(json_schema_validator);
                }

                &v0.json_schema_validator.unwrap()
            }
        };

        let json_value = value
            .try_to_validating_json()
            .map_err(ProtocolError::ValueError)?;

        let res = validator.validate(&json_value);

        match res {
            Ok(_) => Ok(SimpleConsensusValidationResult::default()),
            Err(validation_errors) => {
                let errors: Vec<ConsensusError> =
                    validation_errors.map(ConsensusError::from).collect();

                Ok(SimpleConsensusValidationResult::new_with_errors(errors))
            }
        }
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
