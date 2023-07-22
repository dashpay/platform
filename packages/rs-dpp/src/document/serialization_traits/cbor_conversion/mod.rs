mod v0;

pub use v0::*;
use crate::document::{Document, DocumentV0};
use crate::ProtocolError;
use crate::version::PlatformVersion;
use ciborium::Value as CborValue;

impl DocumentCborMethodsV0 for Document {
    fn from_cbor(document_cbor: &[u8], document_id: Option<[u8; 32]>, owner_id: Option<[u8; 32]>, platform_version: &PlatformVersion) -> Result<Self, ProtocolError> where Self: Sized {
        match platform_version.dpp.document_versions.document_structure_version {
            0 => DocumentV0::from_cbor(document_cbor, document_id, owner_id, platform_version).map(|document| document.into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Document::from_cbor (for document structure)".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    fn to_cbor_value(&self) -> Result<CborValue, ProtocolError> {
        match self { Document::V0(v0) => v0.to_cbor_value() }
    }

    fn to_cbor(&self) -> Result<Vec<u8>, ProtocolError> {
        match self { Document::V0(v0) => v0.to_cbor() }
    }
}