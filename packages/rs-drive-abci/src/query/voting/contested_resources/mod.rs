use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_contested_resources_request::Version as RequestVersion;
use dapi_grpc::platform::v0::get_contested_resources_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{GetContestedResourcesRequest, GetContestedResourcesResponse};
use dpp::version::PlatformVersion;

mod v0;

impl<C> Platform<C> {
    /// Querying of the contested resources
    pub fn query_contested_resources(
        &self,
        GetContestedResourcesRequest { version }: GetContestedResourcesRequest,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetContestedResourcesResponse>, Error> {
        let Some(version) = version else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::DecodingError(
                    "could not decode contested resource vote state query".to_string(),
                ),
            ));
        };

        let feature_version_bounds = &platform_version
            .drive_abci
            .query
            .voting_based_queries
            .contested_resources;

        let feature_version = match &version {
            RequestVersion::V0(_) => 0,
        };
        if !feature_version_bounds.check_version(feature_version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::UnsupportedQueryVersion(
                    "query_contested_resources".to_string(),
                    feature_version_bounds.min_version,
                    feature_version_bounds.max_version,
                    platform_version.protocol_version,
                    feature_version,
                ),
            ));
        }
        match version {
            RequestVersion::V0(request_v0) => {
                let result = self.query_contested_resources_v0(
                    request_v0,
                    platform_state,
                    platform_version,
                )?;

                Ok(result.map(|response_v0| GetContestedResourcesResponse {
                    version: Some(ResponseVersion::V0(response_v0)),
                }))
            }
        }
    }
}
