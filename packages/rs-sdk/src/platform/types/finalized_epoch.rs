//! Finalized epoch related types and helpers
use crate::platform::Query;
use crate::Error;
use dapi_grpc::platform::v0::{get_finalized_epoch_infos_request, GetFinalizedEpochInfosRequest};
use dpp::block::epoch::EpochIndex;

pub use dash_platform_queries::types::finalized_epoch::FinalizedEpochQuery;

impl Query<GetFinalizedEpochInfosRequest> for FinalizedEpochQuery {
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetFinalizedEpochInfosRequest, Error> {
        let prove = settings.prove;
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
    fn query(
        &self,
        settings: &crate::platform::QuerySettings<'_>,
    ) -> Result<GetFinalizedEpochInfosRequest, Error> {
        FinalizedEpochQuery::from(*self).query(settings)
    }
}
