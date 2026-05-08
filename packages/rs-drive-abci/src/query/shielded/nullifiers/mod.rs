mod v0;

use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_shielded_nullifiers_request::Version as RequestVersion;
use dapi_grpc::platform::v0::get_shielded_nullifiers_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{GetShieldedNullifiersRequest, GetShieldedNullifiersResponse};
use dpp::version::PlatformVersion;

impl<C> Platform<C> {
    /// Querying shielded nullifier spend status
    pub fn query_shielded_nullifiers(
        &self,
        GetShieldedNullifiersRequest { version }: GetShieldedNullifiersRequest,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetShieldedNullifiersResponse>, Error> {
        let Some(version) = version else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::DecodingError("could not decode shielded nullifiers query".to_string()),
            ));
        };

        let feature_version_bounds = &platform_version
            .drive_abci
            .query
            .shielded_queries
            .nullifiers;

        let feature_version = match &version {
            RequestVersion::V0(_) => 0,
        };
        if !feature_version_bounds.check_version(feature_version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::UnsupportedQueryVersion(
                    "shielded_nullifiers".to_string(),
                    feature_version_bounds.min_version,
                    feature_version_bounds.max_version,
                    platform_version.protocol_version,
                    feature_version,
                ),
            ));
        }
        match version {
            RequestVersion::V0(request_v0) => {
                let result = self.query_shielded_nullifiers_v0(
                    request_v0,
                    platform_state,
                    platform_version,
                )?;

                Ok(result.map(|response_v0| GetShieldedNullifiersResponse {
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
    use dapi_grpc::platform::v0::get_shielded_nullifiers_request::GetShieldedNullifiersRequestV0;
    use dapi_grpc::platform::v0::get_shielded_nullifiers_response::get_shielded_nullifiers_response_v0;
    use dpp::dashcore::Network;

    #[test]
    fn missing_version_returns_decoding_error() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetShieldedNullifiersRequest { version: None };

        let result = platform
            .query_shielded_nullifiers(request, &state, version)
            .expect("expected query to succeed with validation errors");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::DecodingError(msg)] if msg.contains("shielded nullifiers")
        ));
    }

    #[test]
    fn dispatcher_wraps_v0_response() {
        // The dispatcher must wrap the inner V0 response with Version::V0.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetShieldedNullifiersRequest {
            version: Some(RequestVersion::V0(GetShieldedNullifiersRequestV0 {
                nullifiers: vec![vec![0x11u8; 32]],
                prove: false,
            })),
        };

        let result = platform
            .query_shielded_nullifiers(request, &state, version)
            .expect("expected query to succeed");
        assert!(result.errors.is_empty());

        let response = result.data.expect("expected response data");
        let inner = match response.version {
            Some(ResponseVersion::V0(v0)) => v0,
            other => panic!("expected ResponseVersion::V0, got {:?}", other),
        };
        match inner.result {
            Some(get_shielded_nullifiers_response_v0::Result::NullifierStatuses(statuses)) => {
                assert_eq!(statuses.entries.len(), 1);
                assert!(!statuses.entries[0].is_spent);
            }
            other => panic!("expected NullifierStatuses, got {:?}", other),
        }
    }
}
