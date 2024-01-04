//! Epoch-related types and helpers
use async_trait::async_trait;
use dapi_grpc::platform::v0::GetEpochsInfoRequest;
use dpp::block::{epoch::EpochIndex, extended_epoch_info::ExtendedEpochInfo};

use crate::{
    platform::{Fetch, LimitQuery, Query},
    Error, Sdk,
};

#[async_trait]

/// Helper trait for managing Epoch information
pub trait ExtendedEpochInfoEx: Sized {
    /// Fetch current (the latest) epoch from the platform.
    async fn fetch_current(sdk: &mut Sdk) -> Result<Self, Error>;
}

#[async_trait]
impl ExtendedEpochInfoEx for ExtendedEpochInfo {
    async fn fetch_current(sdk: &mut Sdk) -> Result<Self, Error> {
        let query = LimitQuery {
            query: EpochQuery {
                start: None,
                ascending: false,
            },
            limit: Some(1),
        };

        let epoch = Self::fetch(sdk, query).await?;

        epoch.ok_or(Error::EpochNotFound)
    }
}
/// Query used to fetch multiple epochs from the platform.
#[derive(Clone, Debug)]
pub struct EpochQuery {
    /// Starting number of epoch to fetch.
    ///
    /// It is first returned epoch in the set.
    ///
    /// Value of `None` has the following meaning:
    ///
    /// * if ascending is true, then it is the first epoch on the Platform (eg. epoch 0).
    /// * if ascending is false, then it is the last epoch on the Platform (eg. most recent epoch).
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
