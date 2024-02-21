use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_identities_request::Version;
use dapi_grpc::platform::v0::{GetIdentitiesRequest, GetIdentitiesResponse};
use dapi_grpc::Message;
use dpp::check_validation_result_with_data;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;

mod v0;

impl<C> Platform<C> {
    /// Querying of an identity by a public key hash
    pub fn query_identities(
        &self,
        GetIdentitiesRequest { version }: GetIdentitiesRequest,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetIdentitiesResponse>, Error> {
        let Some(version) = version else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::DecodingError("could not decode identities query".to_string()),
            ));
        };

        let feature_version_bounds = &platform_version
            .drive_abci
            .query
            .identity_based_queries
            .identities;

        let feature_version = match &version {
            Version::V0(_) => 0,
        };
        if !feature_version_bounds.check_version(feature_version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::UnsupportedQueryVersion(
                    "identities".to_string(),
                    feature_version_bounds.min_version,
                    feature_version_bounds.max_version,
                    platform_version.protocol_version,
                    feature_version,
                ),
            ));
        }
        match version {
            Version::V0(request_v0) => self.query_identities_v0(request_v0, platform_version),
        }
    }
}
