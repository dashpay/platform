use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_identity_balance_request::Version;
use dapi_grpc::platform::v0::{GetIdentityBalanceRequest, GetIdentityBalanceResponse};
use dapi_grpc::Message;
use dpp::version::PlatformVersion;

mod v0;

impl<C> Platform<C> {
    /// Querying of an identity by a public key hash
    pub fn query_balance(
        &self,
        GetIdentityBalanceRequest { version }: GetIdentityBalanceRequest,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetIdentityBalanceResponse>, Error> {
        let Some(version) = version else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::DecodingError("could not decode identity balance query".to_string()),
            ));
        };

        let feature_version_bounds = &platform_version
            .drive_abci
            .query
            .identity_based_queries
            .balance;

        let feature_version = match &version {
            Version::V0(_) => 0,
        };
        if !feature_version_bounds.check_version(feature_version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::UnsupportedQueryVersion(
                    "balance".to_string(),
                    feature_version_bounds.min_version,
                    feature_version_bounds.max_version,
                    platform_version.protocol_version,
                    feature_version,
                ),
            ));
        }
        match version {
            Version::V0(request_v0) => self.query_balance_v0(request_v0, platform_version),
        }
    }
}
