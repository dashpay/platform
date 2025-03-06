use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_identities_token_balances_request::Version as RequestVersion;
use dapi_grpc::platform::v0::get_identities_token_balances_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{
    GetIdentitiesTokenBalancesRequest, GetIdentitiesTokenBalancesResponse,
};
use dpp::version::PlatformVersion;
mod v0;

impl<C> Platform<C> {
    /// Querying of an identity's token balances by a public key hash
    pub fn query_identities_token_balances(
        &self,
        GetIdentitiesTokenBalancesRequest { version }: GetIdentitiesTokenBalancesRequest,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetIdentitiesTokenBalancesResponse>, Error> {
        let Some(version) = version else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::DecodingError(
                    "could not decode identity token balances query".to_string(),
                ),
            ));
        };

        let feature_version_bounds = &platform_version
            .drive_abci
            .query
            .token_queries
            .identities_token_balances;

        let feature_version = match &version {
            RequestVersion::V0(_) => 0,
        };
        if !feature_version_bounds.check_version(feature_version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::UnsupportedQueryVersion(
                    "identities_token_balances".to_string(),
                    feature_version_bounds.min_version,
                    feature_version_bounds.max_version,
                    platform_version.protocol_version,
                    feature_version,
                ),
            ));
        }

        match version {
            RequestVersion::V0(request_v0) => {
                let result = self.query_identities_token_balances_v0(
                    request_v0,
                    platform_state,
                    platform_version,
                )?;
                Ok(
                    result.map(|response_v0| GetIdentitiesTokenBalancesResponse {
                        version: Some(ResponseVersion::V0(response_v0)),
                    }),
                )
            }
        }
    }
}
