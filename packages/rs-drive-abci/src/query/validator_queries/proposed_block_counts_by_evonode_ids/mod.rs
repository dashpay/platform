use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_evonodes_proposed_epoch_blocks_by_ids_request::Version as RequestVersion;
use dapi_grpc::platform::v0::get_evonodes_proposed_epoch_blocks_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{
    GetEvonodesProposedEpochBlocksByIdsRequest, GetEvonodesProposedEpochBlocksResponse,
};
use dpp::version::PlatformVersion;

mod v0;

impl<C> Platform<C> {
    /// Querying of the proposed block counts by evonode ids
    pub fn query_proposed_block_counts_by_evonode_ids(
        &self,
        GetEvonodesProposedEpochBlocksByIdsRequest { version }: GetEvonodesProposedEpochBlocksByIdsRequest,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetEvonodesProposedEpochBlocksResponse>, Error> {
        let Some(version) = version else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::DecodingError(
                    "could not decode evonodes proposed block counts by ids query".to_string(),
                ),
            ));
        };

        let feature_version_bounds = &platform_version
            .drive_abci
            .query
            .validator_queries
            .proposed_block_counts_by_evonode_ids;

        let feature_version = match &version {
            RequestVersion::V0(_) => 0,
        };
        if !feature_version_bounds.check_version(feature_version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::UnsupportedQueryVersion(
                    "proposed_block_counts_by_evonode_ids".to_string(),
                    feature_version_bounds.min_version,
                    feature_version_bounds.max_version,
                    platform_version.protocol_version,
                    feature_version,
                ),
            ));
        }
        match version {
            RequestVersion::V0(request_v0) => {
                let result = self.query_proposed_block_counts_by_evonode_ids_v0(
                    request_v0,
                    platform_state,
                    platform_version,
                )?;

                Ok(
                    result.map(|response_v0| GetEvonodesProposedEpochBlocksResponse {
                        version: Some(ResponseVersion::V0(response_v0)),
                    }),
                )
            }
        }
    }
}
