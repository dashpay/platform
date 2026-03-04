use crate::{Error, Sdk};
use async_trait::async_trait;
use dapi_grpc::platform::v0::{Proof, ResponseMetadata};

#[async_trait]

/// Helper trait for fetching current state from Platform without query parameters.
///
/// Implemented for types that have a single "current" value on Platform, such as
/// the current epoch ([`ExtendedEpochInfo`](dpp::block::extended_epoch_info::ExtendedEpochInfo))
/// or total credits in platform.
pub trait FetchCurrent: Sized {
    /// Fetch the current value from Platform.
    async fn fetch_current(sdk: &Sdk) -> Result<Self, Error>;
    /// Fetch the current value from Platform with metadata.
    async fn fetch_current_with_metadata(sdk: &Sdk) -> Result<(Self, ResponseMetadata), Error>;
    /// Fetch the current value from Platform with metadata and proof.
    async fn fetch_current_with_metadata_and_proof(
        sdk: &Sdk,
    ) -> Result<(Self, ResponseMetadata, Proof), Error>;
}
