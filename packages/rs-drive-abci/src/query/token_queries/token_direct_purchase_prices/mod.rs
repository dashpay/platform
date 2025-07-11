use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_token_direct_purchase_prices_request::Version as RequestVersion;
use dapi_grpc::platform::v0::get_token_direct_purchase_prices_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{
    GetTokenDirectPurchasePricesRequest, GetTokenDirectPurchasePricesResponse,
};
use dpp::version::PlatformVersion;
mod v0;

impl<C> Platform<C> {
    /// Retrieve the token direct purchase prices for given token ids
    pub fn query_token_direct_purchase_prices(
        &self,
        GetTokenDirectPurchasePricesRequest { version }: GetTokenDirectPurchasePricesRequest,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetTokenDirectPurchasePricesResponse>, Error> {
        let Some(version) = version else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::DecodingError(
                    "could not decode token direct purchase prices query".to_string(),
                ),
            ));
        };

        let feature_version_bounds = &platform_version
            .drive_abci
            .query
            .token_queries
            .token_direct_purchase_prices;

        let feature_version = match &version {
            RequestVersion::V0(_) => 0,
        };
        if !feature_version_bounds.check_version(feature_version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::UnsupportedQueryVersion(
                    "token_direct_purchase_prices".to_string(),
                    feature_version_bounds.min_version,
                    feature_version_bounds.max_version,
                    platform_version.protocol_version,
                    feature_version,
                ),
            ));
        }

        match version {
            RequestVersion::V0(request_v0) => {
                let result = self.query_token_direct_purchase_prices_v0(
                    request_v0,
                    platform_state,
                    platform_version,
                )?;
                Ok(
                    result.map(|response_v0| GetTokenDirectPurchasePricesResponse {
                        version: Some(ResponseVersion::V0(response_v0)),
                    }),
                )
            }
        }
    }
}
