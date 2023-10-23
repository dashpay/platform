use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dpp::check_validation_result_with_data;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use prost::Message;
use dapi_grpc::platform::v0::get_identities_by_public_key_hashes_request::Version;
use dapi_grpc::platform::v0::GetIdentitiesByPublicKeyHashesRequest;

mod v0;

impl<C> Platform<C> {
    /// Querying of an identity by a public key hash
    pub(in crate::query) fn query_identities_by_public_key_hashes(
        &self,
        state: &PlatformState,
        query_data: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<Vec<u8>>, Error> {
        let GetIdentitiesByPublicKeyHashesRequest { version } =
            check_validation_result_with_data!(GetIdentitiesByPublicKeyHashesRequest::decode(query_data));

        let Some(version) = version else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::DecodingError("could not decode identities by public key hashes query".to_string()),
            ));
        };

        let feature_version_bounds = &platform_version
            .drive_abci
            .query
            .identity_based_queries
            .identities_by_public_key_hashes;

        let feature_version = match &version {
            Version::V0(_) => 0,
        };
        if !feature_version_bounds.check_version(feature_version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::UnsupportedQueryVersion(
                    "identities_by_public_key_hashes".to_string(),
                    feature_version_bounds.min_version,
                    feature_version_bounds.max_version,
                    platform_version.protocol_version,
                    feature_version,
                ),
            ));
        }
        match version {
            Version::V0(get_identity_request) => {
                self.query_identities_by_public_key_hashes_v0(state, get_identity_request, platform_version)
            }
        }
    }
}

