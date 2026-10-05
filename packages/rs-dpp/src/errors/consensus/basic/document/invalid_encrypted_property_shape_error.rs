use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

/// A document supplied a value for a property its type declares `encryptedFor`
/// whose length is not one the declared scheme produces: at least the IV plus
/// one block, and a multiple of the block length. The shape is all consensus
/// can check about a ciphertext.
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
    "Property {property} is declared encrypted under {scheme}, but its {actual_length} bytes are \
     not a ciphertext of that scheme: at least {minimum_length} bytes and a multiple of \
     {block_length} are required"
)]
#[platform_serialize(unversioned)]
pub struct InvalidEncryptedPropertyShapeError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    property: String,
    scheme: String,
    actual_length: u32,
    minimum_length: u32,
    block_length: u32,
}

impl InvalidEncryptedPropertyShapeError {
    pub fn new(
        property: String,
        scheme: String,
        actual_length: u32,
        minimum_length: u32,
        block_length: u32,
    ) -> Self {
        Self {
            property,
            scheme,
            actual_length,
            minimum_length,
            block_length,
        }
    }

    /// The dotted path of the property, as the document type flattens it.
    pub fn property(&self) -> &str {
        &self.property
    }

    /// The wire name of the scheme the property is declared under.
    pub fn scheme(&self) -> &str {
        &self.scheme
    }

    /// The length the document supplied.
    pub fn actual_length(&self) -> u32 {
        self.actual_length
    }

    /// The shortest ciphertext the scheme produces.
    pub fn minimum_length(&self) -> u32 {
        self.minimum_length
    }

    /// The block length a ciphertext of the scheme is a multiple of.
    pub fn block_length(&self) -> u32 {
        self.block_length
    }
}

impl From<InvalidEncryptedPropertyShapeError> for ConsensusError {
    fn from(err: InvalidEncryptedPropertyShapeError) -> Self {
        Self::BasicError(BasicError::InvalidEncryptedPropertyShapeError(err))
    }
}
