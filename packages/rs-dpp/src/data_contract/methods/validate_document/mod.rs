use crate::data_contract::document_type::property_constraints::DocumentSystemValues;
use crate::prelude::DataContract;
use platform_value::Value;
use platform_version::version::PlatformVersion;

#[cfg(all(test, feature = "fixtures-and-mocks"))]
mod duplicate_keys_tests;
mod v0;
mod v1;
mod v2;
use crate::consensus::ConsensusError;
use crate::document::Document;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
pub use v0::*;

/// Whether `error` refuses a document for a byte array or identifier the document serializer
/// cannot read as bytes, a list holding a member that is not a `Value::U8`, which generation 2
/// of `validate_document` refuses once the schema check passes. The refusal reads only the
/// document and its type, so check_tx gives it, and a batch failing it is refused unpaid: a
/// proposer drops it and validators reject a block holding it, as they do while serializing
/// such a document is an internal error.
pub fn is_unreadable_byte_array_refusal(error: &ConsensusError) -> bool {
    v2::is_unreadable_byte_array_refusal(error)
}

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
            1 => self.validate_document_v1(name, document, platform_version),
            2 => self.validate_document_v2(name, document, platform_version),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::validate_document".to_string(),
                known_versions: vec![0, 1, 2],
                received: version,
            }),
        }
    }

    fn validate_document_properties(
        &self,
        name: &str,
        properties: Value,
        system: &DocumentSystemValues,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .methods
            .validate_document
        {
            0 => self.validate_document_properties_v0(name, properties, system, platform_version),
            1 => self.validate_document_properties_v1(name, properties, system, platform_version),
            2 => self.validate_document_properties_v2(name, properties, system, platform_version),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::validate_document_properties".to_string(),
                known_versions: vec![0, 1, 2],
                received: version,
            }),
        }
    }
}
