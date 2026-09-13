use std::fmt::Display;

use super::v0::{Proof, ResponseMetadata};

pub trait VersionedGrpcResponse {
    type Error: Display;
    fn proof(&self) -> Result<&Proof, Self::Error>;

    fn proof_owned(self) -> Result<Proof, Self::Error>;
    fn metadata(&self) -> Result<&ResponseMetadata, Self::Error>;
}

/// A trait for responses that contain merk proof bytes directly instead of a Proof message.
/// Used for branch/chunk state sync responses.
pub trait MerkProofVersionedGrpcResponse {
    type Error: Display;
    fn merk_proof(&self) -> Result<&Vec<u8>, Self::Error>;
    fn merk_proof_owned(self) -> Result<Vec<u8>, Self::Error>;
}

/// A trait representing versioned message with version V.
///
/// Message SomeRequest that supports version 0 should implement VersionedGrpcMessage<SomeRequestV0>.
pub trait VersionedGrpcMessage<V: prost::Message>: From<V> {}
