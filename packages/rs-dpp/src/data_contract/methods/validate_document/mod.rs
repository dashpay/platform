use crate::prelude::DataContract;
use platform_value::Value;
use platform_version::version::PlatformVersion;

mod v0;
use crate::document::Document;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
pub use v0::*;

impl DataContractDocumentValidationMethodsV0 for DataContract {
    fn validate_document(
        &self,
        name: &str,
        document: &Document,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .methods
            .validate_document
        {
            0 => self.validate_document_v0(name, document, platform_version),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::validate_document".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    fn validate_document_properties(
        &self,
        name: &str,
        properties: Value,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .methods
            .validate_document
        {
            0 => self.validate_document_properties_v0(name, properties, platform_version),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::validate_document_properties".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
