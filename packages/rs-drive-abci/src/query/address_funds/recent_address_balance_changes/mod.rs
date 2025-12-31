use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_recent_address_balance_changes_request::Version as RequestVersion;
use dapi_grpc::platform::v0::get_recent_address_balance_changes_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{
    GetRecentAddressBalanceChangesRequest, GetRecentAddressBalanceChangesResponse,
};
use dpp::version::PlatformVersion;

mod v0;

impl<C> Platform<C> {
    /// Querying of recent address balance changes
    pub fn query_recent_address_balance_changes(
        &self,
        GetRecentAddressBalanceChangesRequest { version }: GetRecentAddressBalanceChangesRequest,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetRecentAddressBalanceChangesResponse>, Error> {
        let Some(version) = version else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::DecodingError(
                    "could not decode recent address balance changes query".to_string(),
                ),
            ));
        };

        let feature_version_bounds = &platform_version
            .drive_abci
            .query
            .address_funds_queries
            .recent_address_balance_changes;

        let feature_version = match &version {
            RequestVersion::V0(_) => 0,
        };
        if !feature_version_bounds.check_version(feature_version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::UnsupportedQueryVersion(
                    "recent_address_balance_changes".to_string(),
                    feature_version_bounds.min_version,
                    feature_version_bounds.max_version,
                    platform_version.protocol_version,
                    feature_version,
                ),
            ));
        }

        match version {
            RequestVersion::V0(request_v0) => {
                let result = self.query_recent_address_balance_changes_v0(
                    request_v0,
                    platform_state,
                    platform_version,
                )?;
                Ok(
                    result.map(|response_v0| GetRecentAddressBalanceChangesResponse {
                        version: Some(ResponseVersion::V0(response_v0)),
                    }),
                )
            }
        }
    }
}
