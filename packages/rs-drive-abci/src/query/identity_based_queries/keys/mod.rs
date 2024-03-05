use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_identity_keys_request::Version as RequestVersion;
use dapi_grpc::platform::v0::get_identity_keys_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{GetIdentityKeysRequest, GetIdentityKeysResponse};
use dpp::version::PlatformVersion;

mod v0;

impl<C> Platform<C> {
    /// Querying of an identity's keys
    pub fn query_keys(
        &self,
        GetIdentityKeysRequest { version }: GetIdentityKeysRequest,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetIdentityKeysResponse>, Error> {
        let Some(version) = version else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::DecodingError("could not decode identity keys query".to_string()),
            ));
        };

        let feature_version_bounds = &platform_version
            .drive_abci
            .query
            .identity_based_queries
            .keys;

        let feature_version = match &version {
            RequestVersion::V0(_) => 0,
        };
        if !feature_version_bounds.check_version(feature_version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::UnsupportedQueryVersion(
                    "keys".to_string(),
                    feature_version_bounds.min_version,
                    feature_version_bounds.max_version,
                    platform_version.protocol_version,
                    feature_version,
                ),
            ));
        }
        match version {
            RequestVersion::V0(request_v0) => {
                let result = self.query_keys_v0(request_v0, platform_version)?;

                Ok(result.map(|response_v0| GetIdentityKeysResponse {
                    version: Some(ResponseVersion::V0(response_v0)),
                }))
            }
        }
    }
}
