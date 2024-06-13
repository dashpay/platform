use crate::document::extended_document::v0::ExtendedDocumentV0;
use crate::document::serialization_traits::ExtendedDocumentPlatformConversionMethodsV0;

use crate::prelude::ExtendedDocument;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use bincode::enc::Encoder;
use bincode::error::EncodeError;
use platform_serialization::{PlatformVersionEncode, PlatformVersionedDecode};
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

impl PlatformVersionEncode for ExtendedDocument {
    fn platform_encode<E: Encoder>(
        &self,
        encoder: &mut E,
        platform_version: &PlatformVersion,
    ) -> Result<(), EncodeError> {
        let serialized = self.serialize_to_bytes(platform_version).map_err(|e| {
            EncodeError::OtherString(format!("Failed to serialize ExtendedDocument: {}", e))
        })?;

        serialized.platform_encode(encoder, platform_version)
    }
}

impl PlatformVersionedDecode for ExtendedDocument {
    fn platform_versioned_decode<D: bincode::de::Decoder>(
        decoder: &mut D,
        platform_version: &PlatformVersion,
    ) -> Result<Self, bincode::error::DecodeError> {
        let bytes = Vec::<u8>::platform_versioned_decode(decoder, platform_version)?;

        Self::from_bytes(&bytes, platform_version)
            .map_err(|e| {
                EncodeError::OtherString(format!("Failed to serialize ExtendedDocument: {}", e))
            })
            .map_err(|e| bincode::error::DecodeError::OtherString(e.to_string()))
    }
}
