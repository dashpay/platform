use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_nullifiers_trunk_state_request::Version as RequestVersion;
use dapi_grpc::platform::v0::get_nullifiers_trunk_state_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{GetNullifiersTrunkStateRequest, GetNullifiersTrunkStateResponse};
use dpp::version::PlatformVersion;
mod v0;

impl<C> Platform<C> {
    /// Querying of the nullifiers trunk state (proof for sync)
    pub fn query_nullifiers_trunk_state(
        &self,
        GetNullifiersTrunkStateRequest { version }: GetNullifiersTrunkStateRequest,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetNullifiersTrunkStateResponse>, Error> {
        let Some(version) = version else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::DecodingError(
                    "could not decode nullifiers trunk state query".to_string(),
                ),
            ));
        };

        let feature_version_bounds = &platform_version
            .drive_abci
            .query
            .shielded_queries
            .nullifiers_trunk_state;

        let feature_version = match &version {
            RequestVersion::V0(_) => 0,
        };
        if !feature_version_bounds.check_version(feature_version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::UnsupportedQueryVersion(
                    "nullifiers_trunk_state".to_string(),
                    feature_version_bounds.min_version,
                    feature_version_bounds.max_version,
                    platform_version.protocol_version,
                    feature_version,
                ),
            ));
        }

        match version {
            RequestVersion::V0(request_v0) => {
                let result = self.query_nullifiers_trunk_state_v0(
                    request_v0,
                    platform_state,
                    platform_version,
                )?;
                Ok(result.map(|response_v0| GetNullifiersTrunkStateResponse {
                    version: Some(ResponseVersion::V0(response_v0)),
                }))
            }
        }
    }
}
