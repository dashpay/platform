use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;

use dapi_grpc::platform::v0::get_token_perpetual_distribution_last_claim_request::Version as RequestVersion;
use dapi_grpc::platform::v0::get_token_perpetual_distribution_last_claim_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{
    GetTokenPerpetualDistributionLastClaimRequest, GetTokenPerpetualDistributionLastClaimResponse,
};
use dpp::version::PlatformVersion;

mod v0;

impl<C> Platform<C> {
    /// Query the last perpetual distribution claim for a given token and identity
    pub fn query_token_perpetual_distribution_last_claim(
        &self,
        GetTokenPerpetualDistributionLastClaimRequest { version }: GetTokenPerpetualDistributionLastClaimRequest,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetTokenPerpetualDistributionLastClaimResponse>, Error> {
        let Some(version) = version else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::DecodingError(
                    "could not decode token perpetual distribution last claim query".to_string(),
                ),
            ));
        };

        let feature_version_bounds = &platform_version
            .drive_abci
            .query
            .token_queries
            .token_perpetual_distribution_last_claim;

        let feature_version = match &version {
            RequestVersion::V0(_) => 0,
        };

        if !feature_version_bounds.check_version(feature_version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::UnsupportedQueryVersion(
                    "token_perpetual_distribution_last_claim".to_string(),
                    feature_version_bounds.min_version,
                    feature_version_bounds.max_version,
                    platform_version.protocol_version,
                    feature_version,
                ),
            ));
        }

        match version {
            RequestVersion::V0(request_v0) => {
                let result = self.query_token_perpetual_distribution_last_claim_v0(
                    request_v0,
                    platform_state,
                    platform_version,
                )?;
                Ok(result.map(
                    |response_v0| GetTokenPerpetualDistributionLastClaimResponse {
                        version: Some(ResponseVersion::V0(response_v0)),
                    },
                ))
            }
        }
    }
}
