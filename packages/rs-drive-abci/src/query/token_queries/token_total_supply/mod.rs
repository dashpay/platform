use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_token_total_supply_request::Version as RequestVersion;
use dapi_grpc::platform::v0::get_token_total_supply_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{GetTokenTotalSupplyRequest, GetTokenTotalSupplyResponse};
use dpp::version::PlatformVersion;
mod v0;

impl<C> Platform<C> {
    /// Querying of token total supply
    pub fn query_token_total_supply(
        &self,
        GetTokenTotalSupplyRequest { version }: GetTokenTotalSupplyRequest,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetTokenTotalSupplyResponse>, Error> {
        let Some(version) = version else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::DecodingError(
                    "could not decode identity token total supply query".to_string(),
                ),
            ));
        };

        let feature_version_bounds = &platform_version
            .drive_abci
            .query
            .token_queries
            .token_total_supply;

        let feature_version = match &version {
            RequestVersion::V0(_) => 0,
        };
        if !feature_version_bounds.check_version(feature_version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::UnsupportedQueryVersion(
                    "token_total_supply".to_string(),
                    feature_version_bounds.min_version,
                    feature_version_bounds.max_version,
                    platform_version.protocol_version,
                    feature_version,
                ),
            ));
        }

        match version {
            RequestVersion::V0(request_v0) => {
                let result =
                    self.query_token_total_supply_v0(request_v0, platform_state, platform_version)?;
                Ok(result.map(|response_v0| GetTokenTotalSupplyResponse {
                    version: Some(ResponseVersion::V0(response_v0)),
                }))
            }
        }
    }
}
