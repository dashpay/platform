use crate::data_contract::document_type::DocumentTypeRef;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

pub(in crate::document) trait DocumentPlatformDeserializationMethodsV0 {
    /// Reads a serialized document and creates a Document from it.
    fn from_bytes_v0(
        serialized_document: &[u8],
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}
