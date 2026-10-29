use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_asset_lock_outpoint_info_request::Version as RequestVersion;
use dapi_grpc::platform::v0::get_asset_lock_outpoint_info_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{GetAssetLockOutpointInfoRequest, GetAssetLockOutpointInfoResponse};
use dpp::version::PlatformVersion;

mod v0;

impl<C> Platform<C> {
    /// Query the stored state of an asset-lock outpoint.
    pub fn query_asset_lock_outpoint_info(
        &self,
        GetAssetLockOutpointInfoRequest { version }: GetAssetLockOutpointInfoRequest,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetAssetLockOutpointInfoResponse>, Error> {
        let Some(version) = version else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::DecodingError(
                    "could not decode asset lock outpoint info query".to_string(),
                ),
            ));
        };
        let bounds = &platform_version.drive_abci.query.asset_lock_outpoint_info;
        let feature_version = match &version {
            RequestVersion::V0(_) => 0,
        };
        if !bounds.check_version(feature_version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::UnsupportedQueryVersion(
                    "asset_lock_outpoint_info".to_string(),
                    bounds.min_version,
                    bounds.max_version,
                    platform_version.protocol_version,
                    feature_version,
                ),
            ));
        }
        match version {
            RequestVersion::V0(request) => self
                .query_asset_lock_outpoint_info_v0(request, platform_state, platform_version)
                .map(|result| {
                    result.map(|response| GetAssetLockOutpointInfoResponse {
                        version: Some(ResponseVersion::V0(response)),
                    })
                }),
        }
    }
}
