use crate::data_contract::methods::validation::v0::DataContractValidationMethodsV0;
use crate::prelude::DataContract;
use platform_value::Value;
use platform_version::version::PlatformVersion;

mod v0;
use crate::document::Document;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
pub use v0::*;

impl DataContractValidationMethodsV0 for DataContract {
    fn validate_document(
        &mut self,
        name: &str,
        document: &Document,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version.dpp.contract_versions.methods.validation {
            0 => self.validate_document_v0(name, document, platform_version),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::validate_document".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    fn validate_document_value(
        &mut self,
        name: &str,
        value: &Value,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version.dpp.contract_versions.methods.validation {
            0 => self.validate_document_value_v0(name, value, platform_version),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::validate_document".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
