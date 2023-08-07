use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::serialization_traits::{
    DocumentPlatformConversionMethodsV0, DocumentPlatformSerializationMethodsV0,
};
use crate::document::{Document, DocumentV0};
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_version::version::FeatureVersion;

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
        mut self,
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
}
