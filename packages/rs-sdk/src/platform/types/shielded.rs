//! Shielded pool query types and helpers
use crate::platform::fetch_current_no_parameters::FetchCurrent;
use crate::{platform::Fetch, Error, Sdk};
use async_trait::async_trait;
use dapi_grpc::platform::v0::{Proof, ResponseMetadata};
use drive_proof_verifier::types::{
    MostRecentShieldedAnchor, NoParamQuery, ShieldedAnchors, ShieldedPoolState,
};

#[async_trait]
impl FetchCurrent for ShieldedPoolState {
    async fn fetch_current(sdk: &Sdk) -> Result<Self, Error> {
        let (state, _) = Self::fetch_current_with_metadata(sdk).await?;
        Ok(state)
    }

    async fn fetch_current_with_metadata(sdk: &Sdk) -> Result<(Self, ResponseMetadata), Error> {
        let (state, metadata) = Self::fetch_with_metadata(sdk, NoParamQuery {}, None).await?;
        Ok((
            state.ok_or(Error::Generic("shielded pool state not found".to_string()))?,
            metadata,
        ))
    }

    async fn fetch_current_with_metadata_and_proof(
        sdk: &Sdk,
    ) -> Result<(Self, ResponseMetadata, Proof), Error> {
        let (state, metadata, proof) =
            Self::fetch_with_metadata_and_proof(sdk, NoParamQuery {}, None).await?;
        Ok((
            state.ok_or(Error::Generic("shielded pool state not found".to_string()))?,
            metadata,
            proof,
        ))
    }
}

#[async_trait]
impl FetchCurrent for ShieldedAnchors {
    async fn fetch_current(sdk: &Sdk) -> Result<Self, Error> {
        let (anchors, _) = Self::fetch_current_with_metadata(sdk).await?;
        Ok(anchors)
    }

    async fn fetch_current_with_metadata(sdk: &Sdk) -> Result<(Self, ResponseMetadata), Error> {
        let (anchors, metadata) = Self::fetch_with_metadata(sdk, NoParamQuery {}, None).await?;
        Ok((
            anchors.ok_or(Error::Generic("shielded anchors not found".to_string()))?,
            metadata,
        ))
    }

    async fn fetch_current_with_metadata_and_proof(
        sdk: &Sdk,
    ) -> Result<(Self, ResponseMetadata, Proof), Error> {
        let (anchors, metadata, proof) =
            Self::fetch_with_metadata_and_proof(sdk, NoParamQuery {}, None).await?;
        Ok((
            anchors.ok_or(Error::Generic("shielded anchors not found".to_string()))?,
            metadata,
            proof,
        ))
    }
}

#[async_trait]
impl FetchCurrent for MostRecentShieldedAnchor {
    async fn fetch_current(sdk: &Sdk) -> Result<Self, Error> {
        let (anchor, _) = Self::fetch_current_with_metadata(sdk).await?;
        Ok(anchor)
    }

    async fn fetch_current_with_metadata(sdk: &Sdk) -> Result<(Self, ResponseMetadata), Error> {
        let (anchor, metadata) = Self::fetch_with_metadata(sdk, NoParamQuery {}, None).await?;
        Ok((
            anchor.ok_or(Error::Generic(
                "most recent shielded anchor not set".to_string(),
            ))?,
            metadata,
        ))
    }

    async fn fetch_current_with_metadata_and_proof(
        sdk: &Sdk,
    ) -> Result<(Self, ResponseMetadata, Proof), Error> {
        let (anchor, metadata, proof) =
            Self::fetch_with_metadata_and_proof(sdk, NoParamQuery {}, None).await?;
        Ok((
            anchor.ok_or(Error::Generic(
                "most recent shielded anchor not set".to_string(),
            ))?,
            metadata,
            proof,
        ))
    }
}
