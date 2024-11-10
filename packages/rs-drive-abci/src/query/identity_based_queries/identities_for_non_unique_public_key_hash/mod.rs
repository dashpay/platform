use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_identities_for_non_unique_public_key_hash_request::Version as RequestVersion;
use dapi_grpc::platform::v0::get_identities_for_non_unique_public_key_hash_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{
    GetIdentitiesForNonUniquePublicKeyHashRequest, GetIdentitiesForNonUniquePublicKeyHashResponse,
    GetIdentityByPublicKeyHashResponse,
};
use dpp::version::PlatformVersion;

mod v0;

impl<C> Platform<C> {
    /// Querying of identities for a non-unique public key hash
    pub fn query_identities_for_non_unique_public_key_hash(
        &self,
        GetIdentitiesForNonUniquePublicKeyHashRequest { version }: GetIdentitiesForNonUniquePublicKeyHashRequest,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetIdentitiesForNonUniquePublicKeyHashResponse>, Error> {
        let Some(version) = version else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::DecodingError(
                    "could not decode identities for non unique public key hash query".to_string(),
                ),
            ));
        };

        let feature_version_bounds = &platform_version
            .drive_abci
            .query
            .identity_based_queries
            .identities_for_non_unique_public_key_hash;

        let feature_version = match &version {
            RequestVersion::V0(_) => 0,
        };

        if !feature_version_bounds.check_version(feature_version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::UnsupportedQueryVersion(
                    "identities_for_non_unique_public_key_hash".to_string(),
                    feature_version_bounds.min_version,
                    feature_version_bounds.max_version,
                    platform_version.protocol_version,
                    feature_version,
                ),
            ));
        }

        match version {
            RequestVersion::V0(request_v0) => {
                let request = self.query_identities_for_non_unique_public_key_hash_v0(
                    request_v0,
                    platform_state,
                    platform_version,
                )?;

                Ok(request.map(
                    |response_v0| GetIdentitiesForNonUniquePublicKeyHashResponse {
                        version: Some(ResponseVersion::V0(response_v0)),
                    },
                ))
            }
        }
    }
}
