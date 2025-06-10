use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;

use dapi_grpc::platform::v0::get_finalized_epoch_infos_request::Version as RequestVersion;
use dapi_grpc::platform::v0::get_finalized_epoch_infos_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{GetFinalizedEpochInfosRequest, GetFinalizedEpochInfosResponse};
use dpp::version::PlatformVersion;

mod v0;

impl<C> Platform<C> {
    /// Query finalized epoch information for a given range
    pub fn query_finalized_epoch_infos(
        &self,
        GetFinalizedEpochInfosRequest { version }: GetFinalizedEpochInfosRequest,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetFinalizedEpochInfosResponse>, Error> {
        let Some(version) = version else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::DecodingError(
                    "could not decode finalized epoch infos query".to_string(),
                ),
            ));
        };

        let feature_version_bounds = &platform_version
            .drive_abci
            .query
            .system
            .finalized_epoch_infos;

        let feature_version = match &version {
            RequestVersion::V0(_) => 0,
        };

        if !feature_version_bounds.check_version(feature_version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::UnsupportedQueryVersion(
                    "finalized_epoch_infos".to_string(),
                    feature_version_bounds.min_version,
                    feature_version_bounds.max_version,
                    platform_version.protocol_version,
                    feature_version,
                ),
            ));
        }

        match version {
            RequestVersion::V0(request_v0) => {
                let result = self.query_finalized_epoch_infos_v0(
                    request_v0,
                    platform_state,
                    platform_version,
                )?;
                Ok(result.map(|response_v0| GetFinalizedEpochInfosResponse {
                    version: Some(ResponseVersion::V0(response_v0)),
                }))
            }
        }
    }
}
