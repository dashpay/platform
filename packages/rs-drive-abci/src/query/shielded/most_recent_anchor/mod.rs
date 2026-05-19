mod v0;

use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_most_recent_shielded_anchor_request::Version as RequestVersion;
use dapi_grpc::platform::v0::get_most_recent_shielded_anchor_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{
    GetMostRecentShieldedAnchorRequest, GetMostRecentShieldedAnchorResponse,
};
use dpp::version::PlatformVersion;

impl<C> Platform<C> {
    /// Querying the most recent shielded anchor
    pub fn query_most_recent_shielded_anchor(
        &self,
        GetMostRecentShieldedAnchorRequest { version }: GetMostRecentShieldedAnchorRequest,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetMostRecentShieldedAnchorResponse>, Error> {
        let Some(version) = version else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::DecodingError(
                    "could not decode most recent shielded anchor query".to_string(),
                ),
            ));
        };

        let feature_version_bounds = &platform_version
            .drive_abci
            .query
            .shielded_queries
            .most_recent_anchor;

        let feature_version = match &version {
            RequestVersion::V0(_) => 0,
        };
        if !feature_version_bounds.check_version(feature_version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::UnsupportedQueryVersion(
                    "most_recent_shielded_anchor".to_string(),
                    feature_version_bounds.min_version,
                    feature_version_bounds.max_version,
                    platform_version.protocol_version,
                    feature_version,
                ),
            ));
        }
        match version {
            RequestVersion::V0(request_v0) => {
                let result = self.query_most_recent_shielded_anchor_v0(
                    request_v0,
                    platform_state,
                    platform_version,
                )?;

                Ok(
                    result.map(|response_v0| GetMostRecentShieldedAnchorResponse {
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
    use dapi_grpc::platform::v0::get_most_recent_shielded_anchor_request::GetMostRecentShieldedAnchorRequestV0;
    use dapi_grpc::platform::v0::get_most_recent_shielded_anchor_response::get_most_recent_shielded_anchor_response_v0;
    use dpp::dashcore::Network;

    #[test]
    fn test_query_most_recent_shielded_anchor_with_none_version_returns_decoding_error() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetMostRecentShieldedAnchorRequest { version: None };

        let result = platform
            .query_most_recent_shielded_anchor(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::DecodingError(msg)] if msg.contains("could not decode most recent shielded anchor query")
        ));
    }

    #[test]
    fn test_query_most_recent_shielded_anchor_empty_state_returns_zero_anchor() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetMostRecentShieldedAnchorRequest {
            version: Some(RequestVersion::V0(GetMostRecentShieldedAnchorRequestV0 {
                prove: false,
            })),
        };

        let result = platform
            .query_most_recent_shielded_anchor(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "expected no errors");
        let response = result.data.expect("expected response data");
        let inner = match response.version {
            Some(ResponseVersion::V0(v)) => v,
            _ => panic!("expected v0 response"),
        };
        match inner.result {
            Some(get_most_recent_shielded_anchor_response_v0::Result::Anchor(anchor)) => {
                // Fresh init: the most-recent-anchor subtree is empty, so the
                // implementation returns a 32-byte zero vec sentinel.
                assert_eq!(anchor.len(), 32, "expected 32-byte anchor sentinel");
                assert!(
                    anchor.iter().all(|&b| b == 0),
                    "expected all-zero anchor sentinel"
                );
            }
            other => panic!("expected Anchor result, got {:?}", other),
        }
        assert!(inner.metadata.is_some(), "expected metadata present");
    }

    #[test]
    fn test_query_most_recent_shielded_anchor_empty_state_proof() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetMostRecentShieldedAnchorRequest {
            version: Some(RequestVersion::V0(GetMostRecentShieldedAnchorRequestV0 {
                prove: true,
            })),
        };

        let result = platform
            .query_most_recent_shielded_anchor(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "expected no errors for proof");
        let response = result.data.expect("expected response data");
        let inner = match response.version {
            Some(ResponseVersion::V0(v)) => v,
            _ => panic!("expected v0 response"),
        };
        match inner.result {
            Some(get_most_recent_shielded_anchor_response_v0::Result::Proof(proof)) => {
                assert!(!proof.grovedb_proof.is_empty(), "expected non-empty proof");
            }
            other => panic!("expected Proof result, got {:?}", other),
        }
        assert!(inner.metadata.is_some(), "expected metadata present");
    }
}
