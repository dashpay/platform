use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_identity_balance_and_revision_request::Version as RequestVersion;
use dapi_grpc::platform::v0::get_identity_balance_and_revision_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{
    GetIdentityBalanceAndRevisionRequest, GetIdentityBalanceAndRevisionResponse,
};
use dpp::version::PlatformVersion;

mod v0;

impl<C> Platform<C> {
    /// Querying of an identity by a public key hash
    pub fn query_balance_and_revision(
        &self,
        GetIdentityBalanceAndRevisionRequest { version }: GetIdentityBalanceAndRevisionRequest,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetIdentityBalanceAndRevisionResponse>, Error> {
        let Some(version) = version else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::DecodingError(
                    "could not decode identity balance and revision query".to_string(),
                ),
            ));
        };

        let feature_version_bounds = &platform_version
            .drive_abci
            .query
            .identity_based_queries
            .balance_and_revision;

        let feature_version = match &version {
            RequestVersion::V0(_) => 0,
        };
        if !feature_version_bounds.check_version(feature_version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::UnsupportedQueryVersion(
                    "balance_and_revision".to_string(),
                    feature_version_bounds.min_version,
                    feature_version_bounds.max_version,
                    platform_version.protocol_version,
                    feature_version,
                ),
            ));
        }
        match version {
            RequestVersion::V0(request_v0) => {
                let result = self.query_balance_and_revision_v0(request_v0, platform_version)?;

                Ok(
                    result.map(|response_v0| GetIdentityBalanceAndRevisionResponse {
                        version: Some(ResponseVersion::V0(response_v0)),
                    }),
                )
            }
        }
    }
}
