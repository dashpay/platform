use crate::data_contract::document_type::DocumentTypeRef;
use crate::ProtocolError;

pub trait DocumentPlatformDeserializationMethodsV0 {
    /// Reads a serialized document and creates a Document from it.
    fn from_bytes_v0(
        serialized_document: &[u8],
        document_type: DocumentTypeRef,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}
