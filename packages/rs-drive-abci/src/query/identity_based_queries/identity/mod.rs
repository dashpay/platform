use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_identity_request::Version;
use dapi_grpc::platform::v0::{GetIdentityRequest, GetIdentityResponse};
use dpp::version::PlatformVersion;

mod v0;

impl<C> Platform<C> {
    /// Querying of an identity
    pub fn query_identity(
        &self,
        GetIdentityRequest { version }: GetIdentityRequest,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetIdentityResponse>, Error> {
        let Some(version) = version else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::DecodingError("could not decode identity query".to_string()),
            ));
        };

        let feature_version_bounds = &platform_version
            .drive_abci
            .query
            .identity_based_queries
            .identity;

        let feature_version = match &version {
            Version::V0(_) => 0,
        };
        if !feature_version_bounds.check_version(feature_version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::UnsupportedQueryVersion(
                    "identity".to_string(),
                    feature_version_bounds.min_version,
                    feature_version_bounds.max_version,
                    platform_version.protocol_version,
                    feature_version,
                ),
            ));
        }
        match version {
            Version::V0(request_v0) => self.query_identity_v0(request_v0, platform_version),
        }
    }
}
