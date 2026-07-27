mod v0;

use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_shielded_pool_state_request::Version as RequestVersion;
use dapi_grpc::platform::v0::get_shielded_pool_state_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{GetShieldedPoolStateRequest, GetShieldedPoolStateResponse};
use dpp::version::PlatformVersion;

impl<C> Platform<C> {
    /// Querying the shielded pool state (total balance)
    pub fn query_shielded_pool_state(
        &self,
        GetShieldedPoolStateRequest { version }: GetShieldedPoolStateRequest,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetShieldedPoolStateResponse>, Error> {
        let Some(version) = version else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::DecodingError("could not decode shielded pool state query".to_string()),
            ));
        };

        let feature_version_bounds = &platform_version
            .drive_abci
            .query
            .shielded_queries
            .pool_state;

        let feature_version = match &version {
            RequestVersion::V0(_) => 0,
        };
        if !feature_version_bounds.check_version(feature_version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::UnsupportedQueryVersion(
                    "shielded_pool_state".to_string(),
                    feature_version_bounds.min_version,
                    feature_version_bounds.max_version,
                    platform_version.protocol_version,
                    feature_version,
                ),
            ));
        }
        match version {
            RequestVersion::V0(request_v0) => {
                let result = self.query_shielded_pool_state_v0(
                    request_v0,
                    platform_state,
                    platform_version,
                )?;

                Ok(result.map(|response_v0| GetShieldedPoolStateResponse {
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
    use dapi_grpc::platform::v0::get_shielded_pool_state_request::GetShieldedPoolStateRequestV0;
    use dapi_grpc::platform::v0::get_shielded_pool_state_response::get_shielded_pool_state_response_v0;
    use dpp::dashcore::Network;

    #[test]
    fn test_query_shielded_pool_state_with_none_version_returns_decoding_error() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetShieldedPoolStateRequest { version: None };

        let result = platform
            .query_shielded_pool_state(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::DecodingError(msg)] if msg.contains("could not decode shielded pool state query")
        ));
    }

    #[test]
    fn test_query_shielded_pool_state_empty_state_returns_zero_balance() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetShieldedPoolStateRequest {
            version: Some(RequestVersion::V0(GetShieldedPoolStateRequestV0 {
                prove: false,
            })),
        };

        let result = platform
            .query_shielded_pool_state(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "expected no errors");
        let response = result.data.expect("expected response data");
        let inner = match response.version {
            Some(ResponseVersion::V0(v)) => v,
            _ => panic!("expected v0 response"),
        };
        match inner.result {
            Some(get_shielded_pool_state_response_v0::Result::TotalBalance(balance)) => {
                // Freshly initialized state: no credits have been shielded.
                assert_eq!(balance, 0, "expected zero total balance on fresh state");
            }
            other => panic!("expected TotalBalance result, got {:?}", other),
        }
        assert!(inner.metadata.is_some(), "expected metadata present");
    }

    #[test]
    fn test_query_shielded_pool_state_empty_state_proof() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetShieldedPoolStateRequest {
            version: Some(RequestVersion::V0(GetShieldedPoolStateRequestV0 {
                prove: true,
            })),
        };

        let result = platform
            .query_shielded_pool_state(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "expected no errors for proof");
        let response = result.data.expect("expected response data");
        let inner = match response.version {
            Some(ResponseVersion::V0(v)) => v,
            _ => panic!("expected v0 response"),
        };
        match inner.result {
            Some(get_shielded_pool_state_response_v0::Result::Proof(proof)) => {
                assert!(!proof.grovedb_proof.is_empty(), "expected non-empty proof");
            }
            other => panic!("expected Proof result, got {:?}", other),
        }
        assert!(inner.metadata.is_some(), "expected metadata present");
    }
}
