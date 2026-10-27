use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_contract_moderation_entries_request::Version as RequestVersion;
use dapi_grpc::platform::v0::get_contract_moderation_entries_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{
    GetContractModerationEntriesRequest, GetContractModerationEntriesResponse,
};
use dpp::version::PlatformVersion;

mod v0;

impl<C> Platform<C> {
    /// Querying of one page of a moderated contract's banlist or suspension list.
    pub fn query_contract_moderation_entries(
        &self,
        GetContractModerationEntriesRequest { version }: GetContractModerationEntriesRequest,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetContractModerationEntriesResponse>, Error> {
        let Some(version) = version else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::DecodingError(
                    "could not decode contract moderation entries query".to_string(),
                ),
            ));
        };

        let feature_version_bounds = &platform_version
            .drive_abci
            .query
            .contract_moderation_queries
            .contract_moderation_entries;

        let feature_version = match &version {
            RequestVersion::V0(_) => 0,
        };
        if !feature_version_bounds.check_version(feature_version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::UnsupportedQueryVersion(
                    "contract_moderation_entries".to_string(),
                    feature_version_bounds.min_version,
                    feature_version_bounds.max_version,
                    platform_version.protocol_version,
                    feature_version,
                ),
            ));
        }
        match version {
            RequestVersion::V0(request_v0) => {
                let result = self.query_contract_moderation_entries_v0(
                    request_v0,
                    platform_state,
                    platform_version,
                )?;

                Ok(
                    result.map(|response_v0| GetContractModerationEntriesResponse {
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
            .query_contract_moderation_entries(
                GetContractModerationEntriesRequest { version: None },
                &state,
                version,
            )
            .expect("expected query to succeed");
        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::DecodingError(_)]
        ));
    }

    #[test]
    fn should_wrap_the_version_0_response() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let result = platform
            .query_contract_moderation_entries(
                GetContractModerationEntriesRequest {
                    version: Some(RequestVersion::V0(Default::default())),
                },
                &state,
                version,
            )
            .expect("expected query to succeed");
        // An empty request is refused by the version 0 handler, which proves it was reached.
        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(_)]
        ));
    }
}
