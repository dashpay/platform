mod v0;

use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_shielded_notes_count_request::Version as RequestVersion;
use dapi_grpc::platform::v0::get_shielded_notes_count_response::{
    GetShieldedNotesCountResponseV0, Version as ResponseVersion,
};
use dapi_grpc::platform::v0::{GetShieldedNotesCountRequest, GetShieldedNotesCountResponse};
use dpp::version::PlatformVersion;

impl<C> Platform<C> {
    /// Returns the total number of notes currently stored in the
    /// shielded credit pool's CommitmentTree (its leaf count).
    ///
    /// Intended to seed a wallet progress-bar denominator at the start
    /// of a shielded sync. Supports both unproved and proved modes: the
    /// count is the first field (`total_count`) of the serialized
    /// `CommitmentTree` element, whose bytes are bound into the Merk
    /// value hash, so it is provable via a PathQuery proof of that
    /// element — see `GetShieldedNotesCountResponse` in `platform.proto`.
    pub fn query_shielded_notes_count(
        &self,
        GetShieldedNotesCountRequest { version }: GetShieldedNotesCountRequest,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetShieldedNotesCountResponse>, Error> {
        let Some(version) = version else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::DecodingError(
                    "could not decode shielded notes count query".to_string(),
                ),
            ));
        };

        let feature_version_bounds = &platform_version
            .drive_abci
            .query
            .shielded_queries
            .notes_count;

        let feature_version = match &version {
            RequestVersion::V0(_) => 0,
        };
        if !feature_version_bounds.check_version(feature_version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::UnsupportedQueryVersion(
                    "shielded_notes_count".to_string(),
                    feature_version_bounds.min_version,
                    feature_version_bounds.max_version,
                    platform_version.protocol_version,
                    feature_version,
                ),
            ));
        }
        match version {
            RequestVersion::V0(request_v0) => {
                let result = self.query_shielded_notes_count_v0(
                    request_v0,
                    platform_state,
                    platform_version,
                )?;

                Ok(result.map(|response_v0: GetShieldedNotesCountResponseV0| {
                    GetShieldedNotesCountResponse {
                        version: Some(ResponseVersion::V0(response_v0)),
                    }
                }))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::setup_platform;
    use dapi_grpc::platform::v0::get_shielded_notes_count_request::GetShieldedNotesCountRequestV0;
    use dapi_grpc::platform::v0::get_shielded_notes_count_response::get_shielded_notes_count_response_v0;
    use dpp::dashcore::Network;

    #[test]
    fn test_query_shielded_notes_count_with_none_version_returns_decoding_error() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetShieldedNotesCountRequest { version: None };

        let result = platform
            .query_shielded_notes_count(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::DecodingError(msg)] if msg.contains("could not decode shielded notes count query")
        ));
    }

    #[test]
    fn test_query_shielded_notes_count_empty_state_returns_zero() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetShieldedNotesCountRequest {
            version: Some(RequestVersion::V0(GetShieldedNotesCountRequestV0 {
                prove: false,
            })),
        };

        let result = platform
            .query_shielded_notes_count(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "expected no errors");
        let response = result.data.expect("expected response data");
        let inner = match response.version {
            Some(ResponseVersion::V0(v)) => v,
            _ => panic!("expected v0 response"),
        };
        match inner.result {
            Some(get_shielded_notes_count_response_v0::Result::TotalNotesCount(count)) => {
                assert_eq!(count, 0, "expected zero notes on fresh state");
            }
            other => panic!("expected TotalNotesCount result, got {:?}", other),
        }
        assert!(inner.metadata.is_some(), "expected metadata present");
    }

    #[test]
    fn test_query_shielded_notes_count_empty_state_proof() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetShieldedNotesCountRequest {
            version: Some(RequestVersion::V0(GetShieldedNotesCountRequestV0 {
                prove: true,
            })),
        };

        let result = platform
            .query_shielded_notes_count(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "expected no errors for proof");
        let response = result.data.expect("expected response data");
        let inner = match response.version {
            Some(ResponseVersion::V0(v)) => v,
            _ => panic!("expected v0 response"),
        };
        match inner.result {
            Some(get_shielded_notes_count_response_v0::Result::Proof(proof)) => {
                assert!(!proof.grovedb_proof.is_empty(), "expected non-empty proof");
            }
            other => panic!("expected Proof result, got {:?}", other),
        }
        assert!(inner.metadata.is_some(), "expected metadata present");
    }
}
