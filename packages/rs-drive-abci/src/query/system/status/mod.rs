mod v0;

use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_status_request::{GetStatusRequestV0, Version as RequestVersion};
use dapi_grpc::platform::v0::get_status_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{GetStatusRequest, GetStatusResponse};
use dpp::version::PlatformVersion;

impl<C> Platform<C> {
    /// Querying Drive information for platform status endpoint
    /// implemented in DAPI
    pub fn query_partial_status(
        &self,
        _request: GetStatusRequest,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetStatusResponse>, Error> {
        // GetStatusRequestV0 doesn't contain any fields so request version
        // will be always empty
        let version = RequestVersion::V0(GetStatusRequestV0 {});

        let feature_version_bounds = &platform_version.drive_abci.query.system.partial_status;

        let feature_version = match &version {
            RequestVersion::V0(_) => 0,
        };
        if !feature_version_bounds.check_version(feature_version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::UnsupportedQueryVersion(
                    "partial_status".to_string(),
                    feature_version_bounds.min_version,
                    feature_version_bounds.max_version,
                    platform_version.protocol_version,
                    feature_version,
                ),
            ));
        }
        match version {
            RequestVersion::V0(request_v0) => {
                let result = self.query_partial_status_v0(request_v0, platform_state)?;

                Ok(result.map(|response_v0| GetStatusResponse {
                    version: Some(ResponseVersion::V0(response_v0)),
                }))
            }
        }
    }
}
