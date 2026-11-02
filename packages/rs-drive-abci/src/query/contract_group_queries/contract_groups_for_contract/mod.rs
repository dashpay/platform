use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_contract_groups_for_contract_request::Version as RequestVersion;
use dapi_grpc::platform::v0::get_contract_groups_for_contract_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{
    GetContractGroupsForContractRequest, GetContractGroupsForContractResponse,
};
use dpp::version::PlatformVersion;

mod v0;

impl<C> Platform<C> {
    /// Querying of the contract groups a contract belongs to, as a whole, through its document
    /// types and through its tokens.
    pub fn query_contract_groups_for_contract(
        &self,
        GetContractGroupsForContractRequest { version }: GetContractGroupsForContractRequest,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetContractGroupsForContractResponse>, Error> {
        let Some(version) = version else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::DecodingError(
                    "could not decode contract groups for contract query".to_string(),
                ),
            ));
        };

        let feature_version_bounds = &platform_version
            .drive_abci
            .query
            .contract_group_queries
            .contract_groups_for_contract;

        let feature_version = match &version {
            RequestVersion::V0(_) => 0,
        };
        if !feature_version_bounds.check_version(feature_version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::UnsupportedQueryVersion(
                    "contract_groups_for_contract".to_string(),
                    feature_version_bounds.min_version,
                    feature_version_bounds.max_version,
                    platform_version.protocol_version,
                    feature_version,
                ),
            ));
        }
        match version {
            RequestVersion::V0(request_v0) => {
                let result = self.query_contract_groups_for_contract_v0(
                    request_v0,
                    platform_state,
                    platform_version,
                )?;

                Ok(
                    result.map(|response_v0| GetContractGroupsForContractResponse {
                        version: Some(ResponseVersion::V0(response_v0)),
                    }),
                )
            }
        }
    }
}
