use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_contract_settled_deletions_request::Version as RequestVersion;
use dapi_grpc::platform::v0::get_contract_settled_deletions_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{
    GetContractSettledDeletionsRequest, GetContractSettledDeletionsResponse,
};
use dpp::version::PlatformVersion;

mod v0;

impl<C> Platform<C> {
    /// Querying of the approvals a contract\'s seated moderation team gave the deletion of settled
    /// documents
    pub fn query_contract_settled_deletions(
        &self,
        GetContractSettledDeletionsRequest { version }: GetContractSettledDeletionsRequest,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetContractSettledDeletionsResponse>, Error> {
        let Some(version) = version else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::DecodingError(
                    "could not decode contract settled deletions query".to_string(),
                ),
            ));
        };

        let feature_version_bounds = &platform_version
            .drive_abci
            .query
            .contract_moderation_queries
            .contract_settled_deletions;

        let feature_version = match &version {
            RequestVersion::V0(_) => 0,
        };
        if !feature_version_bounds.check_version(feature_version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::UnsupportedQueryVersion(
                    "contract_settled_deletions".to_string(),
                    feature_version_bounds.min_version,
                    feature_version_bounds.max_version,
                    platform_version.protocol_version,
                    feature_version,
                ),
            ));
        }
        match version {
            RequestVersion::V0(request_v0) => {
                let result = self.query_contract_settled_deletions_v0(
                    request_v0,
                    platform_state,
                    platform_version,
                )?;

                Ok(
                    result.map(|response_v0| GetContractSettledDeletionsResponse {
                        version: Some(ResponseVersion::V0(response_v0)),
                    }),
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::setup_platform;
    use dpp::dashcore::Network;

    #[test]
    fn should_refuse_a_request_without_a_version() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let result = platform
            .query_contract_settled_deletions(
                GetContractSettledDeletionsRequest { version: None },
                &state,
                version,
            )
            .expect("expected the query to run");
        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::DecodingError(_)]
        ));
    }
}
