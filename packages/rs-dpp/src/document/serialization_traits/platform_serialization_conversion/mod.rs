pub(in crate::document) mod deserialize;
pub(in crate::document) mod serialize;
mod v0;

use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::{Document, DocumentV0};
#[cfg(feature = "validation")]
use crate::prelude::ConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::{FeatureVersion, PlatformVersion};
pub use v0::*;

impl DocumentPlatformConversionMethodsV0 for Document {
    /// Serializes the document.
    ///
    /// The serialization of a document follows the pattern:
    /// id 32 bytes + owner_id 32 bytes + encoded values byte arrays
    fn serialize(
        &self,
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        match self {
            Document::V0(document_v0) => document_v0.serialize(document_type, platform_version),
        }
    }

    fn serialize_specific_version(
        &self,
        document_type: DocumentTypeRef,
        feature_version: FeatureVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        match self {
            Document::V0(document_v0) => {
                document_v0.serialize_specific_version(document_type, feature_version)
            }
        }
    }

    /// Serializes and consumes the document.
    ///
    /// The serialization of a document follows the pattern:
    /// id 32 bytes + owner_id 32 bytes + encoded values byte arrays
    fn serialize_consume(
        self,
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        match self {
            Document::V0(document_v0) => {
                document_v0.serialize_consume(document_type, platform_version)
            }
        }
    }

    /// Reads a serialized document and creates a Document from it.
    fn from_bytes(
        serialized_document: &[u8],
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .document_versions
            .document_structure_version
        {
            0 => Ok(
                DocumentV0::from_bytes(serialized_document, document_type, platform_version)?
                    .into(),
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Document::from_bytes".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    #[cfg(feature = "validation")]
    fn from_bytes_in_consensus(
        serialized_document: &[u8],
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<Self>, ProtocolError>
    where
        Self: Sized,
    {
        match platform_version
            .dpp
            .document_versions
            .document_structure_version
        {
            0 => Ok(DocumentV0::from_bytes_in_consensus(
                serialized_document,
                document_type,
                platform_version,
            )?
            .map(|document_v0| document_v0.into())),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Document::from_bytes_in_consensus".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::data_contract::accessors::v0::DataContractV0Getters;
    use crate::data_contract::document_type::random_document::CreateRandomDocument;
    use crate::document::serialization_traits::DocumentPlatformConversionMethodsV0;
    use crate::document::Document;
    use crate::tests::json_document::json_document_to_contract;
    use platform_version::version::PlatformVersion;

    #[test]
    fn test_serialization() {
        let platform_version = PlatformVersion::first();
        let contract = json_document_to_contract(
            "../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract.json",
            false,
            platform_version,
        )
        .expect("expected to get dashpay contract");

        let document_type = contract
            .document_type_for_name("contactRequest")
            .expect("expected to get profile document type");
        let document = document_type
            .random_document(Some(3333), platform_version)
            .expect("expected to get a random document");

        let serialized_document = document
            .serialize(document_type, platform_version)
            .expect("expected to serialize");

        let deserialized_document = Document::from_bytes(
            serialized_document.as_slice(),
            document_type,
            platform_version,
        )
        .expect("expected to deserialize a document");
        assert_eq!(document, deserialized_document);
        for _i in 0..10000 {
            let document = document_type
                .random_document(Some(3333), platform_version)
                .expect("expected to get a random document");
            document
                .serialize_consume(document_type, platform_version)
                .expect("expected to serialize consume");
        }
    }
}
