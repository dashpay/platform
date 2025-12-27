use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_addresses_branch_state_request::Version as RequestVersion;
use dapi_grpc::platform::v0::get_addresses_branch_state_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{GetAddressesBranchStateRequest, GetAddressesBranchStateResponse};
use dpp::version::PlatformVersion;
mod v0;

impl<C> Platform<C> {
    /// Querying of the addresses branch state (merk proof for sync)
    pub fn query_addresses_branch_state(
        &self,
        GetAddressesBranchStateRequest { version }: GetAddressesBranchStateRequest,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetAddressesBranchStateResponse>, Error> {
        let Some(version) = version else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::DecodingError(
                    "could not decode addresses branch state query".to_string(),
                ),
            ));
        };

        let feature_version_bounds = &platform_version
            .drive_abci
            .query
            .address_funds_queries
            .addresses_branch_state;

        let feature_version = match &version {
            RequestVersion::V0(_) => 0,
        };
        if !feature_version_bounds.check_version(feature_version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::UnsupportedQueryVersion(
                    "addresses_branch_state".to_string(),
                    feature_version_bounds.min_version,
                    feature_version_bounds.max_version,
                    platform_version.protocol_version,
                    feature_version,
                ),
            ));
        }

        match version {
            RequestVersion::V0(request_v0) => {
                let result = self.query_addresses_branch_state_v0(
                    request_v0,
                    platform_state,
                    platform_version,
                )?;
                Ok(result.map(|response_v0| GetAddressesBranchStateResponse {
                    version: Some(ResponseVersion::V0(response_v0)),
                }))
            }
        }
    }
}
