use crate::document::extended_document::v0::ExtendedDocumentV0;
use crate::document::serialization_traits::ExtendedDocumentPlatformConversionMethodsV0;

use crate::prelude::ExtendedDocument;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_version::version::FeatureVersion;

impl ExtendedDocumentPlatformConversionMethodsV0 for ExtendedDocument {
    /// Serializes the document.
    ///
    /// The serialization of a document follows the pattern:
    /// id 32 bytes + owner_id 32 bytes + encoded values byte arrays
    fn serialize_to_bytes(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        match self {
            ExtendedDocument::V0(document_v0) => document_v0.serialize_to_bytes(platform_version),
        }
    }

    fn serialize_specific_version_to_bytes(
        &self,
        feature_version: FeatureVersion,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        match self {
            ExtendedDocument::V0(document_v0) => {
                document_v0.serialize_specific_version_to_bytes(feature_version, platform_version)
            }
        }
    }

    /// Serializes and consumes the document.
    ///
    /// The serialization of a document follows the pattern:
    /// id 32 bytes + owner_id 32 bytes + encoded values byte arrays
    fn serialize_consume_to_bytes(
        self,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        match self {
            ExtendedDocument::V0(document_v0) => {
                document_v0.serialize_consume_to_bytes(platform_version)
            }
        }
    }

    /// Reads a serialized document and creates a Document from it.
    fn from_bytes(
        serialized_document: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .document_versions
            .extended_document_structure_version
        {
            0 => Ok(ExtendedDocumentV0::from_bytes(serialized_document, platform_version)?.into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "ExtendedDocument::from_bytes (structure)".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
