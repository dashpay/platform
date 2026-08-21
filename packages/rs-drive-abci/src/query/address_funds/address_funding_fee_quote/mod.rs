use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_address_funding_fee_quote_request::Version as RequestVersion;
use dapi_grpc::platform::v0::get_address_funding_fee_quote_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{
    GetAddressFundingFeeQuoteRequest, GetAddressFundingFeeQuoteResponse,
};
use dpp::version::PlatformVersion;
pub(crate) mod v0;

impl<C> Platform<C> {
    /// Querying of a state-aware address funding fee quote
    pub fn query_address_funding_fee_quote(
        &self,
        GetAddressFundingFeeQuoteRequest { version }: GetAddressFundingFeeQuoteRequest,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetAddressFundingFeeQuoteResponse>, Error> {
        let Some(version) = version else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::DecodingError(
                    "could not decode address funding fee quote query".to_string(),
                ),
            ));
        };

        let feature_version_bounds = &platform_version
            .drive_abci
            .query
            .address_funds_queries
            .address_funding_fee_quote;

        let feature_version = match &version {
            RequestVersion::V0(_) => 0,
        };
        if !feature_version_bounds.check_version(feature_version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::UnsupportedQueryVersion(
                    "address_funding_fee_quote".to_string(),
                    feature_version_bounds.min_version,
                    feature_version_bounds.max_version,
                    platform_version.protocol_version,
                    feature_version,
                ),
            ));
        }

        match version {
            RequestVersion::V0(request_v0) => {
                let result = self.query_address_funding_fee_quote_v0(
                    request_v0,
                    platform_state,
                    platform_version,
                )?;
                Ok(result.map(|response_v0| GetAddressFundingFeeQuoteResponse {
                    version: Some(ResponseVersion::V0(response_v0)),
                }))
            }
        }
    }
}
