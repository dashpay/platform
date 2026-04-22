use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_nullifiers_branch_state_request::Version as RequestVersion;
use dapi_grpc::platform::v0::get_nullifiers_branch_state_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{GetNullifiersBranchStateRequest, GetNullifiersBranchStateResponse};
use dpp::version::PlatformVersion;
mod v0;

impl<C> Platform<C> {
    /// Querying of the nullifiers branch state (merk proof for sync)
    pub fn query_nullifiers_branch_state(
        &self,
        GetNullifiersBranchStateRequest { version }: GetNullifiersBranchStateRequest,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetNullifiersBranchStateResponse>, Error> {
        let Some(version) = version else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::DecodingError(
                    "could not decode nullifiers branch state query".to_string(),
                ),
            ));
        };

        let feature_version_bounds = &platform_version
            .drive_abci
            .query
            .shielded_queries
            .nullifiers_branch_state;

        let feature_version = match &version {
            RequestVersion::V0(_) => 0,
        };
        if !feature_version_bounds.check_version(feature_version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::UnsupportedQueryVersion(
                    "nullifiers_branch_state".to_string(),
                    feature_version_bounds.min_version,
                    feature_version_bounds.max_version,
                    platform_version.protocol_version,
                    feature_version,
                ),
            ));
        }

        match version {
            RequestVersion::V0(request_v0) => {
                let result = self.query_nullifiers_branch_state_v0(
                    request_v0,
                    platform_state,
                    platform_version,
                )?;
                Ok(result.map(|response_v0| GetNullifiersBranchStateResponse {
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
    use dapi_grpc::platform::v0::get_nullifiers_branch_state_request::GetNullifiersBranchStateRequestV0;
    use dpp::dashcore::Network;

    #[test]
    fn missing_version_returns_decoding_error() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetNullifiersBranchStateRequest { version: None };

        let result = platform
            .query_nullifiers_branch_state(request, &state, version)
            .expect("expected query to succeed with validation errors");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::DecodingError(msg)]
                if msg.contains("nullifiers branch state")
        ));
    }

    #[test]
    fn invalid_input_bubbles_up_through_dispatcher() {
        // Confirm that when the v0 handler returns Err (e.g. for out-of-range depth),
        // the dispatcher propagates it as an Err rather than swallowing it.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetNullifiersBranchStateRequest {
            version: Some(RequestVersion::V0(GetNullifiersBranchStateRequestV0 {
                pool_type: 0,
                pool_identifier: vec![],
                key: vec![0u8; 32],
                depth: 2, // below min_depth = 6
                checkpoint_height: 0,
            })),
        };

        let err = platform
            .query_nullifiers_branch_state(request, &state, version)
            .expect_err("expected error from out-of-range depth");

        assert!(matches!(err, Error::Drive(_)));
    }
}
