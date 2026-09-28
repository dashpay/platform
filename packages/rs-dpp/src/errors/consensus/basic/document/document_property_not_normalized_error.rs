use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

/// A `normalizedFrom` string property of the written document is not the normalized form of
/// the property it names: its value differs from the source's normalized form, or it is
/// present while the source is absent (or absent while the source is present, which the
/// platform only sees when a document skipped the fill it runs on arrival).
///
/// A pure structure check on document create and replace (protocol version 14): it reads
/// the document alone, so it is a basic error, not a state one.
#[derive(
    Error,
    Debug,
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    DecodeUntrusted,
)]
#[error(
    "Document type \"{document_type_name}\" property \"{property}\" must be the \
     {transform} form of property \"{source_property}\", and absent when it is"
)]
#[platform_serialize(unversioned)]
pub struct DocumentPropertyNotNormalizedError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_type_name: String,
    /// Dotted path of the declaring property within the document type.
    property: String,
    /// Dotted path of the property it is normalized from.
    source_property: String,
    /// The transform's wire name, such as `homographSafeASCII`.
    transform: String,
}

impl DocumentPropertyNotNormalizedError {
    pub fn new(
        document_type_name: String,
        property: String,
        source_property: String,
        transform: String,
    ) -> Self {
        Self {
            document_type_name,
            property,
            source_property,
            transform,
        }
    }

    pub fn document_type_name(&self) -> &str {
        &self.document_type_name
    }

    pub fn property(&self) -> &str {
        &self.property
    }

    pub fn source_property(&self) -> &str {
        &self.source_property
    }

    pub fn transform(&self) -> &str {
        &self.transform
    }
}

impl From<DocumentPropertyNotNormalizedError> for ConsensusError {
    fn from(err: DocumentPropertyNotNormalizedError) -> Self {
        Self::BasicError(BasicError::DocumentPropertyNotNormalizedError(err))
    }
}
