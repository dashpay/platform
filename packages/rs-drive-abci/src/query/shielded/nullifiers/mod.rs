mod v0;

use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_shielded_nullifiers_request::Version as RequestVersion;
use dapi_grpc::platform::v0::get_shielded_nullifiers_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{GetShieldedNullifiersRequest, GetShieldedNullifiersResponse};
use dpp::version::PlatformVersion;

impl<C> Platform<C> {
    /// Querying shielded nullifier spend status
    pub fn query_shielded_nullifiers(
        &self,
        GetShieldedNullifiersRequest { version }: GetShieldedNullifiersRequest,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetShieldedNullifiersResponse>, Error> {
        let Some(version) = version else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::DecodingError("could not decode shielded nullifiers query".to_string()),
            ));
        };

        let feature_version_bounds = &platform_version
            .drive_abci
            .query
            .shielded_queries
            .nullifiers;

        let feature_version = match &version {
            RequestVersion::V0(_) => 0,
        };
        if !feature_version_bounds.check_version(feature_version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::UnsupportedQueryVersion(
                    "shielded_nullifiers".to_string(),
                    feature_version_bounds.min_version,
                    feature_version_bounds.max_version,
                    platform_version.protocol_version,
                    feature_version,
                ),
            ));
        }
        match version {
            RequestVersion::V0(request_v0) => {
                let result = self.query_shielded_nullifiers_v0(
                    request_v0,
                    platform_state,
                    platform_version,
                )?;

                Ok(result.map(|response_v0| GetShieldedNullifiersResponse {
                    version: Some(ResponseVersion::V0(response_v0)),
                }))
            }
        }
    }
}
