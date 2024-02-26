use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_identity_by_public_key_hash_request::Version as RequestVersion;
use dapi_grpc::platform::v0::get_identity_by_public_key_hash_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{
    GetIdentityByPublicKeyHashRequest, GetIdentityByPublicKeyHashResponse,
};
use dpp::version::PlatformVersion;

mod v0;

impl<C> Platform<C> {
    /// Querying of an identity by a public key hash
    pub fn query_identity_by_public_key_hash(
        &self,
        GetIdentityByPublicKeyHashRequest { version }: GetIdentityByPublicKeyHashRequest,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetIdentityByPublicKeyHashResponse>, Error> {
        let Some(version) = version else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::DecodingError(
                    "could not decode identity by public key hash query".to_string(),
                ),
            ));
        };

        let feature_version_bounds = &platform_version
            .drive_abci
            .query
            .identity_based_queries
            .identity_by_public_key_hash;

        let feature_version = match &version {
            RequestVersion::V0(_) => 0,
        };

        if !feature_version_bounds.check_version(feature_version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::UnsupportedQueryVersion(
                    "identity_by_public_key_hash".to_string(),
                    feature_version_bounds.min_version,
                    feature_version_bounds.max_version,
                    platform_version.protocol_version,
                    feature_version,
                ),
            ));
        }

        match version {
            RequestVersion::V0(request_v0) => {
                let request =
                    self.query_identity_by_public_key_hash_v0(request_v0, platform_version)?;

                Ok(
                    request.map(|response_v0| GetIdentityByPublicKeyHashResponse {
                        version: Some(ResponseVersion::V0(response_v0)),
                    }),
                )
            }
        }
    }
}
