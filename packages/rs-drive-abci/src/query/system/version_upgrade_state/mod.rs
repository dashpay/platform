mod v0;

use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_protocol_version_upgrade_state_request::Version as RequestVersion;
use dapi_grpc::platform::v0::get_protocol_version_upgrade_state_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{
    GetProtocolVersionUpgradeStateRequest, GetProtocolVersionUpgradeStateResponse,
};
use dapi_grpc::Message;
use dpp::version::PlatformVersion;

impl<C> Platform<C> {
    /// Querying of version upgrade state
    pub fn query_version_upgrade_state(
        &self,
        GetProtocolVersionUpgradeStateRequest { version }: GetProtocolVersionUpgradeStateRequest,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetProtocolVersionUpgradeStateResponse>, Error> {
        let Some(version) = version else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::DecodingError("could not decode identity keys query".to_string()),
            ));
        };

        let feature_version_bounds = &platform_version
            .drive_abci
            .query
            .system
            .version_upgrade_state;

        let feature_version = match &version {
            RequestVersion::V0(_) => 0,
        };
        if !feature_version_bounds.check_version(feature_version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::UnsupportedQueryVersion(
                    "version_upgrade_state".to_string(),
                    feature_version_bounds.min_version,
                    feature_version_bounds.max_version,
                    platform_version.protocol_version,
                    feature_version,
                ),
            ));
        }
        match version {
            RequestVersion::V0(request_v0) => {
                let result = self.query_version_upgrade_state_v0(request_v0, platform_version)?;

                Ok(
                    result.map(|response_v0| GetProtocolVersionUpgradeStateResponse {
                        version: Some(ResponseVersion::V0(response_v0)),
                    }),
                )
            }
        }
    }
}
