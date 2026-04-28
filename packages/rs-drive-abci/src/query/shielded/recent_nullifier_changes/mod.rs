use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_recent_nullifier_changes_request::Version as RequestVersion;
use dapi_grpc::platform::v0::get_recent_nullifier_changes_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{
    GetRecentNullifierChangesRequest, GetRecentNullifierChangesResponse,
};
use dpp::version::PlatformVersion;

mod v0;

impl<C> Platform<C> {
    /// Querying of recent nullifier changes
    pub fn query_recent_nullifier_changes(
        &self,
        GetRecentNullifierChangesRequest { version }: GetRecentNullifierChangesRequest,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetRecentNullifierChangesResponse>, Error> {
        let Some(version) = version else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::DecodingError(
                    "could not decode recent nullifier changes query".to_string(),
                ),
            ));
        };

        let feature_version_bounds = &platform_version
            .drive_abci
            .query
            .shielded_queries
            .recent_nullifier_changes;

        let feature_version = match &version {
            RequestVersion::V0(_) => 0,
        };
        if !feature_version_bounds.check_version(feature_version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::UnsupportedQueryVersion(
                    "recent_nullifier_changes".to_string(),
                    feature_version_bounds.min_version,
                    feature_version_bounds.max_version,
                    platform_version.protocol_version,
                    feature_version,
                ),
            ));
        }

        match version {
            RequestVersion::V0(request_v0) => {
                let result = self.query_recent_nullifier_changes_v0(
                    request_v0,
                    platform_state,
                    platform_version,
                )?;
                Ok(result.map(|response_v0| GetRecentNullifierChangesResponse {
                    version: Some(ResponseVersion::V0(response_v0)),
                }))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::setup_platform;
    use dapi_grpc::platform::v0::get_recent_nullifier_changes_request::GetRecentNullifierChangesRequestV0;
    use dapi_grpc::platform::v0::get_recent_nullifier_changes_response::get_recent_nullifier_changes_response_v0;
    use dpp::dashcore::Network;

    #[test]
    fn missing_version_returns_decoding_error() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetRecentNullifierChangesRequest { version: None };

        let result = platform
            .query_recent_nullifier_changes(request, &state, version)
            .expect("expected query to succeed with validation errors");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::DecodingError(msg)]
                if msg.contains("recent nullifier changes")
        ));
    }

    #[test]
    fn dispatcher_wraps_v0_response() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetRecentNullifierChangesRequest {
            version: Some(RequestVersion::V0(GetRecentNullifierChangesRequestV0 {
                start_height: 0,
                prove: true,
            })),
        };

        let result = platform
            .query_recent_nullifier_changes(request, &state, version)
            .expect("expected query to succeed");
        assert!(result.errors.is_empty());

        let response = result.data.expect("expected response data");
        let inner = match response.version {
            Some(ResponseVersion::V0(v0)) => v0,
            other => panic!("expected ResponseVersion::V0, got {:?}", other),
        };
        assert!(matches!(
            inner.result,
            Some(get_recent_nullifier_changes_response_v0::Result::Proof(_))
        ));
        assert!(inner.metadata.is_some());
    }
}
