use crate::{Error, Sdk};
use async_trait::async_trait;
use dapi_grpc::platform::v0::{Proof, ResponseMetadata};

#[async_trait]

/// Helper trait for managing Epoch information
pub trait FetchCurrent: Sized {
    /// Fetch current (the latest) epoch from Platform.
    async fn fetch_current(sdk: &Sdk) -> Result<Self, Error>;
    /// Fetch current (the latest) epoch from Platform with metadata.
    async fn fetch_current_with_metadata(sdk: &Sdk) -> Result<(Self, ResponseMetadata), Error>;
    /// Fetch current (the latest) epoch from Platform with metadata and proof.
    async fn fetch_current_with_metadata_and_proof(
        sdk: &Sdk,
    ) -> Result<(Self, ResponseMetadata, Proof), Error>;
}
