//! Finalized epoch related types and helpers
use crate::platform::{LimitQuery, Query};
use crate::Error;
use dapi_grpc::platform::v0::{get_finalized_epoch_infos_request, GetFinalizedEpochInfosRequest};
use dpp::block::epoch::EpochIndex;

/// Query used to fetch multiple finalized epochs from Platform.
#[derive(Clone, Debug)]
pub struct FinalizedEpochQuery {
    /// Starting epoch index.
    pub start_epoch_index: EpochIndex,
    /// Whether to include the start epoch.
    pub start_epoch_index_included: bool,
    /// Ending epoch index.
    pub end_epoch_index: EpochIndex,
    /// Whether to include the end epoch.
    pub end_epoch_index_included: bool,
}

impl Default for FinalizedEpochQuery {
    fn default() -> Self {
        Self {
            start_epoch_index: 0,
            start_epoch_index_included: true,
            end_epoch_index: 0,
            end_epoch_index_included: false,
        }
    }
}

impl From<(EpochIndex, EpochIndex)> for FinalizedEpochQuery {
    fn from((start, end): (EpochIndex, EpochIndex)) -> Self {
        Self {
            start_epoch_index: start,
            start_epoch_index_included: true,
            end_epoch_index: end,
            end_epoch_index_included: true,
        }
    }
}

impl Query<GetFinalizedEpochInfosRequest> for FinalizedEpochQuery {
    fn query(self, prove: bool) -> Result<GetFinalizedEpochInfosRequest, Error> {
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }
        Ok(GetFinalizedEpochInfosRequest {
            version: Some(get_finalized_epoch_infos_request::Version::V0(
                get_finalized_epoch_infos_request::GetFinalizedEpochInfosRequestV0 {
                    prove,
                    start_epoch_index: self.start_epoch_index as u32,
                    start_epoch_index_included: self.start_epoch_index_included,
                    end_epoch_index: self.end_epoch_index as u32,
                    end_epoch_index_included: self.end_epoch_index_included,
                },
            )),
        })
    }
}

impl Query<GetFinalizedEpochInfosRequest> for (EpochIndex, EpochIndex) {
    fn query(self, prove: bool) -> Result<GetFinalizedEpochInfosRequest, Error> {
        FinalizedEpochQuery::from(self).query(prove)
    }
}

impl Query<GetFinalizedEpochInfosRequest> for LimitQuery<FinalizedEpochQuery> {
    fn query(self, prove: bool) -> Result<GetFinalizedEpochInfosRequest, Error> {
        // For finalized epochs, the limit is handled by the range itself
        // so we just pass through the query
        self.query.query(prove)
    }
}
