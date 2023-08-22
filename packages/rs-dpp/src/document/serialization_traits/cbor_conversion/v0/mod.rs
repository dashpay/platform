use crate::version::PlatformVersion;
use crate::ProtocolError;
use ciborium::Value as CborValue;

pub trait DocumentCborMethodsV0 {
    /// Reads a CBOR-serialized document and creates a Document from it.
    /// If Document and Owner IDs are provided, they are used, otherwise they are created.
    fn from_cbor(
        document_cbor: &[u8],
        document_id: Option<[u8; 32]>,
        owner_id: Option<[u8; 32]>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
    fn to_cbor_value(&self) -> Result<CborValue, ProtocolError>;
    /// Serializes the Document to CBOR.
    fn to_cbor(&self) -> Result<Vec<u8>, ProtocolError>;
}
