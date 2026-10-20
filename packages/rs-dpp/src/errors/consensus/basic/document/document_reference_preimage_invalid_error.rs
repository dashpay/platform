use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

/// A document being created cannot assemble the preimage of a `refersTo` key it
/// reveals, the `findBy` function
/// (`"<referenced property>": { "function": "sys.hash.sha256d", "params": [...] }`): a value
/// a param reads is absent or of a kind a param cannot take, or a variable-length value holds
/// the one-byte separator that follows it, so the preimage would not split back into its
/// params one way.
///
/// A pure structure check on document create (protocol version 14): it reads the transition
/// alone, so it is a basic error, and it refuses the create before the lookup reads state.
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
    "Document type \"{document_type_name}\" reference at \"{path}\" cannot reveal its findBy \
     key: property \"{property}\": {reason}"
)]
#[platform_serialize(unversioned)]
pub struct DocumentReferencePreimageInvalidError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_type_name: String,
    /// The reference declaring the `findBy`: a property path, `$ownerId` or `$creatorId`,
    /// followed by the leaf of a reference expression when it sits in one.
    path: String,
    /// The property whose value is at fault.
    property: String,
    reason: String,
}

impl DocumentReferencePreimageInvalidError {
    pub fn new(document_type_name: String, path: String, property: String, reason: String) -> Self {
        Self {
            document_type_name,
            path,
            property,
            reason,
        }
    }

    pub fn document_type_name(&self) -> &str {
        &self.document_type_name
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn property(&self) -> &str {
        &self.property
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }
}

impl From<DocumentReferencePreimageInvalidError> for ConsensusError {
    fn from(err: DocumentReferencePreimageInvalidError) -> Self {
        Self::BasicError(BasicError::DocumentReferencePreimageInvalidError(err))
    }
}
