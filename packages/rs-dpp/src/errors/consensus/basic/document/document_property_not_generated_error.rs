use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

/// A `generatedFrom` property of the written document is not what its function generates
/// from its parameters: its value differs, or it is present while a parameter is absent (or
/// absent while every parameter is present, which the platform only sees when a document
/// skipped the generation it runs on arrival).
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
    "Document type \"{document_type_name}\" property \"{property}\" must be what {function} \
     generates from {}, and absent when any of them is",
    .params.join(", ")
)]
#[platform_serialize(unversioned)]
pub struct DocumentPropertyNotGeneratedError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_type_name: String,
    /// Dotted path of the declaring property within the document type.
    property: String,
    /// The function's wire name, such as `sys.stringTransformations.homographSafeASCII`.
    function: String,
    /// Dotted paths of the properties the function reads, in order.
    params: Vec<String>,
}

impl DocumentPropertyNotGeneratedError {
    pub fn new(
        document_type_name: String,
        property: String,
        function: String,
        params: Vec<String>,
    ) -> Self {
        Self {
            document_type_name,
            property,
            function,
            params,
        }
    }

    pub fn document_type_name(&self) -> &str {
        &self.document_type_name
    }

    pub fn property(&self) -> &str {
        &self.property
    }

    pub fn function(&self) -> &str {
        &self.function
    }

    pub fn params(&self) -> &[String] {
        &self.params
    }
}

impl From<DocumentPropertyNotGeneratedError> for ConsensusError {
    fn from(err: DocumentPropertyNotGeneratedError) -> Self {
        Self::BasicError(BasicError::DocumentPropertyNotGeneratedError(err))
    }
}
