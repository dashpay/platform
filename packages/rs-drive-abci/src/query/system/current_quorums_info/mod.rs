mod v0;

use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_current_quorums_info_request::Version as RequestVersion;
use dapi_grpc::platform::v0::get_current_quorums_info_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{GetCurrentQuorumsInfoRequest, GetCurrentQuorumsInfoResponse};
use dpp::version::PlatformVersion;

impl<C> Platform<C> {
    /// Querying of current quorums info
    pub fn query_current_quorums_info(
        &self,
        GetCurrentQuorumsInfoRequest { version }: GetCurrentQuorumsInfoRequest,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetCurrentQuorumsInfoResponse>, Error> {
        let Some(version) = version else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::DecodingError("could not decode epoch info request".to_string()),
            ));
        };

        let feature_version_bounds = &platform_version.drive_abci.query.system.epoch_infos;

        let feature_version = match &version {
            RequestVersion::V0(_) => 0,
        };
        if !feature_version_bounds.check_version(feature_version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::UnsupportedQueryVersion(
                    "current_quorums_info".to_string(),
                    feature_version_bounds.min_version,
                    feature_version_bounds.max_version,
                    platform_version.protocol_version,
                    feature_version,
                ),
            ));
        }
        match version {
            RequestVersion::V0(request_v0) => {
                let result = self.query_current_quorums_info_v0(request_v0, platform_state)?;

                Ok(result.map(|response_v0| GetCurrentQuorumsInfoResponse {
                    version: Some(ResponseVersion::V0(response_v0)),
                }))
            }
        }
    }
}
