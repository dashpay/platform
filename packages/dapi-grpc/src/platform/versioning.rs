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

// The metadata proof is always present in proved history responses, including absence.
impl VersionedGrpcResponse for super::v0::GetDocumentHistoryResponse {
    type Error = platform_version::error::PlatformVersionError;

    fn proof(&self) -> Result<&Proof, Self::Error> {
        use super::v0::get_document_history_response::{
            get_document_history_response_v0::Result as ResultV0, Version,
        };
        match &self.version {
            Some(Version::V0(response)) => match &response.result {
                Some(ResultV0::Proof(proof)) => Some(proof),
                _ => None,
            },
            Some(Version::V1(response)) => response.metadata_proof.as_ref(),
            None => None,
        }
        .ok_or_else(|| {
            Self::Error::UnknownVersionError("document history response has no proof".to_owned())
        })
    }

    fn proof_owned(self) -> Result<Proof, Self::Error> {
        use super::v0::get_document_history_response::{
            get_document_history_response_v0::Result as ResultV0, Version,
        };
        match self.version {
            Some(Version::V0(response)) => match response.result {
                Some(ResultV0::Proof(proof)) => Some(proof),
                _ => None,
            },
            Some(Version::V1(response)) => response.metadata_proof,
            None => None,
        }
        .ok_or_else(|| {
            Self::Error::UnknownVersionError("document history response has no proof".to_owned())
        })
    }

    fn metadata(&self) -> Result<&ResponseMetadata, Self::Error> {
        use super::v0::get_document_history_response::Version;
        match &self.version {
            Some(Version::V0(response)) => response.metadata.as_ref(),
            Some(Version::V1(response)) => response.metadata.as_ref(),
            None => None,
        }
        .ok_or_else(|| {
            Self::Error::UnknownVersionError("document history response has no metadata".to_owned())
        })
    }
}
