use crate::data_contract::document_type::DocumentTypeRef;
use crate::data_contract::errors::DataContractError;

#[cfg(feature = "validation")]
use crate::prelude::ConsensusValidationResult;

use crate::prelude::DataContract;

use crate::ProtocolError;

use crate::document::serialization_traits::deserialize::v0::DocumentPlatformDeserializationMethodsV0;
use crate::document::serialization_traits::serialize::v0::DocumentPlatformSerializationMethodsV0;
use crate::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use crate::document::v0::DocumentV0;
use crate::version::PlatformVersion;
use integer_encoding::VarIntReader;

use platform_version::version::FeatureVersion;

use crate::consensus::basic::decode::DecodingError;
#[cfg(feature = "validation")]
use crate::consensus::basic::BasicError;
#[cfg(feature = "validation")]
use crate::consensus::ConsensusError;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::config::DataContractConfig;

mod v0;
mod v1;
mod v2;
mod v3;

// Each serialization format generation lives in its own file (v0.rs–v3.rs);
// the trait impls below are one-line dispatch shims into the inherent
// methods those files define. Consensus discipline: a shipped format's file
// must never change, and a diff touching one is immediately suspect.
impl DocumentPlatformSerializationMethodsV0 for DocumentV0 {
    /// Format 0 — implementation in [`v0`].
    fn serialize_v0(&self, document_type: DocumentTypeRef) -> Result<Vec<u8>, ProtocolError> {
        DocumentV0::serialize_v0(self, document_type)
    }

    /// Format 1 — implementation in [`v1`].
    fn serialize_v1(&self, document_type: DocumentTypeRef) -> Result<Vec<u8>, ProtocolError> {
        DocumentV0::serialize_v1(self, document_type)
    }

    /// Format 2 — implementation in [`v2`].
    fn serialize_v2(&self, document_type: DocumentTypeRef) -> Result<Vec<u8>, ProtocolError> {
        DocumentV0::serialize_v2(self, document_type)
    }

    /// Format 3 — implementation in [`v3`].
    fn serialize_v3(&self, document_type: DocumentTypeRef) -> Result<Vec<u8>, ProtocolError> {
        DocumentV0::serialize_v3(self, document_type)
    }
}

impl DocumentPlatformDeserializationMethodsV0 for DocumentV0 {
    /// Format 0 — implementation in [`v0`].
    fn from_bytes_v0(
        serialized_document: &[u8],
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DataContractError> {
        DocumentV0::from_bytes_v0(serialized_document, document_type, platform_version)
    }

    /// Format 1 — implementation in [`v1`].
    fn from_bytes_v1(
        serialized_document: &[u8],
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DataContractError> {
        DocumentV0::from_bytes_v1(serialized_document, document_type, platform_version)
    }

    /// Format 2 — implementation in [`v2`].
    fn from_bytes_v2(
        serialized_document: &[u8],
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DataContractError> {
        DocumentV0::from_bytes_v2(serialized_document, document_type, platform_version)
    }

    /// Format 3 — implementation in [`v3`].
    fn from_bytes_v3(
        serialized_document: &[u8],
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DataContractError> {
        DocumentV0::from_bytes_v3(serialized_document, document_type, platform_version)
    }
}

impl DocumentPlatformConversionMethodsV0 for DocumentV0 {
    /// Serializes the document.
    ///
    /// The serialization of a document follows the pattern:
    /// id 32 bytes + owner_id 32 bytes + encoded values byte arrays
    fn serialize(
        &self,
        document_type: DocumentTypeRef,
        contract: &DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        if matches!(contract, DataContract::V0(_))
            || matches!(contract.config(), DataContractConfig::V0(_))
        {
            // Any data contract in version 0 should always serialize documents in version 0
            // This is because integers in such a data contract if made through normal versioning should always
            // be i64
            // While it's possible in theory maybe that they are not i64 using serialize_v0
            // will encode all integers as i64.
            self.serialize_v0(document_type)
        } else {
            match platform_version
                .dpp
                .document_versions
                .document_serialization_version
                .default_current_version
            {
                0 => self.serialize_v0(document_type),
                // Version 1 coincides with protocol version 9, which contains tokens, new document types,
                // and most importantly different integer types.
                // Document types now have properties that are known to be things like u8, i32 etc.
                1 => self.serialize_v1(document_type),
                2 => self.serialize_v2(document_type),
                // Version 3 coincides with protocol version 14: it stamps the
                // document with the contract version its bytes conform to,
                // enabling `requiredSince` properties.
                3 => self.serialize_v3(document_type),
                version => Err(ProtocolError::UnknownVersionMismatch {
                    method: "DocumentV0::serialize".to_string(),
                    known_versions: vec![0, 1, 2, 3],
                    received: version,
                }),
            }
        }
    }

    fn serialize_specific_version(
        &self,
        document_type: DocumentTypeRef,
        contract: &DataContract,
        feature_version: FeatureVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        if (matches!(contract, DataContract::V0(_))
            || matches!(contract.config(), DataContractConfig::V0(_)))
            && feature_version != 0
        {
            // Any data contract in version 0 should always serialize documents in version 0
            // This is because integers in such a data contract if made through normal versioning should always
            // be i64
            // While it's possible in theory maybe that they are not i64 using serialize_v0
            // will encode all integers as i64.
            return Err(ProtocolError::NotSupported("Serializing with data contract version 0 or data contract config version 0 is not supported outside of feature version 0".to_string()));
        };
        match feature_version {
            0 => self.serialize_v0(document_type),
            1 => self.serialize_v1(document_type),
            2 => self.serialize_v2(document_type),
            3 => self.serialize_v3(document_type),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentV0::serialize".to_string(),
                known_versions: vec![0, 1, 2, 3],
                received: version,
            }),
        }
    }

    /// Reads a serialized document and creates a DocumentV0 from it.
    fn from_bytes(
        mut serialized_document: &[u8],
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let serialized_version = serialized_document.read_varint().map_err(|_| {
            DataContractError::DecodingDocumentError(DecodingError::new(
                "error reading revision from serialized document for revision".to_string(),
            ))
        })?;
        match serialized_version {
            0 => {
                match DocumentV0::from_bytes_v0(
                    serialized_document,
                    document_type,
                    platform_version,
                )
                .map_err(ProtocolError::DataContractError)
                {
                    Ok(document) => Ok(document),
                    Err(first_err) => {
                        // let's try decoding in V1 just to be safe
                        // Version 0 will decode all integers as I64
                        // Version 1 will decode all integers properly
                        // When version was 0 used (protocol version 1 to 8) integers other than I64
                        // existed, but were probably never used, which is why we try v1 just to be safe
                        match DocumentV0::from_bytes_v1(
                            serialized_document,
                            document_type,
                            platform_version,
                        ) {
                            Ok(document_from_version_1_deserialization) => {
                                Ok(document_from_version_1_deserialization)
                            }
                            Err(_) => Err(first_err),
                        }
                    }
                }
            }
            1 => DocumentV0::from_bytes_v1(serialized_document, document_type, platform_version)
                .map_err(ProtocolError::DataContractError),
            2 => DocumentV0::from_bytes_v2(serialized_document, document_type, platform_version)
                .map_err(ProtocolError::DataContractError),
            3 => DocumentV0::from_bytes_v3(serialized_document, document_type, platform_version)
                .map_err(ProtocolError::DataContractError),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Document::from_bytes (deserialization)".to_string(),
                known_versions: vec![0, 1, 2, 3],
                received: version,
            }),
        }
    }

    /// Reads a serialized document and creates a DocumentV0 from it.
    #[cfg(feature = "validation")]
    fn from_bytes_in_consensus(
        mut serialized_document: &[u8],
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<Self>, ProtocolError> {
        let serialized_version = serialized_document.read_varint().map_err(|_| {
            DataContractError::DecodingDocumentError(DecodingError::new(
                "error reading revision from serialized document for revision".to_string(),
            ))
        })?;
        match serialized_version {
            0 => {
                match DocumentV0::from_bytes_v0(
                    serialized_document,
                    document_type,
                    platform_version,
                ) {
                    Ok(document) => Ok(ConsensusValidationResult::new_with_data(document)),
                    Err(first_err) => {
                        // let's try decoding in V1 just to be safe
                        // Version 0 will decode all integers as I64
                        // Version 1 will decode all integers properly
                        // When version was 0 used (protocol version 1 to 8) integers other than I64
                        // existed, but were probably never used, which is why we try v1 just to be safe
                        match DocumentV0::from_bytes_v1(
                            serialized_document,
                            document_type,
                            platform_version,
                        ) {
                            Ok(document_from_version_1_deserialization) => {
                                Ok(ConsensusValidationResult::new_with_data(
                                    document_from_version_1_deserialization,
                                ))
                            }
                            Err(_) => Ok(ConsensusValidationResult::new_with_error(
                                ConsensusError::BasicError(BasicError::ContractError(first_err)),
                            )),
                        }
                    }
                }
            }
            1 => {
                match DocumentV0::from_bytes_v1(
                    serialized_document,
                    document_type,
                    platform_version,
                ) {
                    Ok(document) => Ok(ConsensusValidationResult::new_with_data(document)),
                    Err(err) => Ok(ConsensusValidationResult::new_with_error(
                        ConsensusError::BasicError(BasicError::ContractError(err)),
                    )),
                }
            }
            2 => {
                match DocumentV0::from_bytes_v2(
                    serialized_document,
                    document_type,
                    platform_version,
                ) {
                    Ok(document) => Ok(ConsensusValidationResult::new_with_data(document)),
                    Err(err) => Ok(ConsensusValidationResult::new_with_error(
                        ConsensusError::BasicError(BasicError::ContractError(err)),
                    )),
                }
            }
            3 => {
                match DocumentV0::from_bytes_v3(
                    serialized_document,
                    document_type,
                    platform_version,
                ) {
                    Ok(document) => Ok(ConsensusValidationResult::new_with_data(document)),
                    Err(err) => Ok(ConsensusValidationResult::new_with_error(
                        ConsensusError::BasicError(BasicError::ContractError(err)),
                    )),
                }
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Document::from_bytes (deserialization)".to_string(),
                known_versions: vec![0, 1, 2, 3],
                received: version,
            }),
        }
    }
}

#[cfg(test)]
mod tests;
