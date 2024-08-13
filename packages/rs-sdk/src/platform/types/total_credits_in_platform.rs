//! Epoch-related types and helpers
use crate::platform::fetch_current_no_parameters::FetchCurrent;
use crate::{platform::Fetch, Error, Sdk};
use async_trait::async_trait;
use dapi_grpc::platform::v0::{Proof, ResponseMetadata};
use drive_proof_verifier::types::{NoParamQuery, TotalCreditsOnPlatform};

#[async_trait]
impl FetchCurrent for TotalCreditsOnPlatform {
    async fn fetch_current(sdk: &Sdk) -> Result<Self, Error> {
        let (total_credits_on_platform, _) = Self::fetch_current_with_metadata(sdk).await?;
        Ok(total_credits_on_platform)
    }

    async fn fetch_current_with_metadata(sdk: &Sdk) -> Result<(Self, ResponseMetadata), Error> {
        let (total_credits_on_platform, metadata) =
            Self::fetch_with_metadata(sdk, NoParamQuery {}, None).await?;

        Ok((
            total_credits_on_platform.ok_or(Error::TotalCreditsNotFound)?,
            metadata,
        ))
    }

    async fn fetch_current_with_metadata_and_proof(
        sdk: &Sdk,
    ) -> Result<(Self, ResponseMetadata, Proof), Error> {
        let (epoch, metadata, proof) =
            Self::fetch_with_metadata_and_proof(sdk, NoParamQuery {}, None).await?;

        Ok((epoch.ok_or(Error::TotalCreditsNotFound)?, metadata, proof))
    }
}
