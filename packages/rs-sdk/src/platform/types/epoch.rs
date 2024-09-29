//! Epoch-related types and helpers
use async_trait::async_trait;
use dapi_grpc::platform::v0::{GetEpochsInfoRequest, Proof, ResponseMetadata};
use dpp::block::{epoch::EpochIndex, extended_epoch_info::ExtendedEpochInfo};

use crate::platform::fetch_current_no_parameters::FetchCurrent;
use crate::{
    platform::{Fetch, LimitQuery, Query},
    Error, Sdk,
};

/// Epoch type used in the SDK.
pub type Epoch = ExtendedEpochInfo;

#[async_trait]
impl FetchCurrent for ExtendedEpochInfo {
    async fn fetch_current(sdk: &Sdk) -> Result<Self, Error> {
        let (epoch, _) = Self::fetch_current_with_metadata(sdk).await?;
        Ok(epoch)
    }

    async fn fetch_current_with_metadata(sdk: &Sdk) -> Result<(Self, ResponseMetadata), Error> {
        let query = LimitQuery {
            query: EpochQuery {
                start: None,
                ascending: false,
            },
            limit: Some(1),
            start_info: None,
        };

        let (epoch, metadata) = Self::fetch_with_metadata(sdk, query, None).await?;

        Ok((epoch.ok_or(Error::EpochNotFound)?, metadata))
    }

    async fn fetch_current_with_metadata_and_proof(
        sdk: &Sdk,
    ) -> Result<(Self, ResponseMetadata, Proof), Error> {
        let query = LimitQuery {
            query: EpochQuery {
                start: None,
                ascending: false,
            },
            limit: Some(1),
            start_info: None,
        };

        let (epoch, metadata, proof) =
            Self::fetch_with_metadata_and_proof(sdk, query, None).await?;

        Ok((epoch.ok_or(Error::EpochNotFound)?, metadata, proof))
    }
}
/// Query used to fetch multiple epochs from Platform.
#[derive(Clone, Debug)]
pub struct EpochQuery {
    /// Starting number of epoch to fetch.
    ///
    /// It is first returned epoch in the set.
    ///
    /// Value of `None` has the following meaning:
    ///
    /// * if ascending is true, then it is the first epoch on Platform (eg. epoch 0).
    /// * if ascending is false, then it is the last epoch on Platform (eg. most recent epoch).
    pub start: Option<EpochIndex>,
    /// Sort order. Default is ascending (true), which means that the first returned epoch is the oldest one.
    pub ascending: bool,
}

impl Default for EpochQuery {
    fn default() -> Self {
        Self {
            start: None,
            ascending: true,
        }
    }
}

impl From<EpochIndex> for EpochQuery {
    fn from(start: EpochIndex) -> Self {
        Self {
            start: Some(start),
            ascending: true,
        }
    }
}

impl Query<GetEpochsInfoRequest> for EpochQuery {
    fn query(self, prove: bool) -> Result<GetEpochsInfoRequest, Error> {
        LimitQuery::from(self).query(prove)
    }
}
