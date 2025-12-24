use crate::data_contract::document_type::DocumentTypeRef;
use crate::data_contract::errors::DataContractError;
#[cfg(feature = "extended-document")]
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

pub(in crate::document) trait DocumentPlatformDeserializationMethodsV0 {
    /// Reads a serialized document and creates a Document from it.
    /// Version 0 will always decode integers as i64s,
    /// as all integers were stored as i64 in version 0
    fn from_bytes_v0(
        serialized_document: &[u8],
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DataContractError>
    where
        Self: Sized;

    /// Reads a serialized document and creates a Document from it.
    /// Version 1 properly uses the data contract encoded integer types
    fn from_bytes_v1(
        serialized_document: &[u8],
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DataContractError>
    where
        Self: Sized;

    /// Reads a serialized document and creates a Document from it.
    /// Version 2 has the creator id.
    fn from_bytes_v2(
        serialized_document: &[u8],
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DataContractError>
    where
        Self: Sized;
}

#[cfg(feature = "extended-document")]
pub(in crate::document) trait ExtendedDocumentPlatformDeserializationMethodsV0 {
    /// Reads a serialized document and creates a Document from it.
    /// Version 0 will always decode integers as i64s,
    /// as all integers were stored as i64 in version 0
    fn from_bytes_v0(
        serialized_document: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}
