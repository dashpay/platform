mod v0;

use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_shielded_encrypted_notes_request::Version as RequestVersion;
use dapi_grpc::platform::v0::get_shielded_encrypted_notes_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{
    GetShieldedEncryptedNotesRequest, GetShieldedEncryptedNotesResponse,
};
use dpp::version::PlatformVersion;

impl<C> Platform<C> {
    /// Querying shielded encrypted notes for wallet sync
    pub fn query_shielded_encrypted_notes(
        &self,
        GetShieldedEncryptedNotesRequest { version }: GetShieldedEncryptedNotesRequest,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetShieldedEncryptedNotesResponse>, Error> {
        let Some(version) = version else {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::DecodingError(
                    "could not decode shielded encrypted notes query".to_string(),
                ),
            ));
        };

        let feature_version_bounds = &platform_version
            .drive_abci
            .query
            .shielded_queries
            .encrypted_notes;

        let feature_version = match &version {
            RequestVersion::V0(_) => 0,
        };
        if !feature_version_bounds.check_version(feature_version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::UnsupportedQueryVersion(
                    "shielded_encrypted_notes".to_string(),
                    feature_version_bounds.min_version,
                    feature_version_bounds.max_version,
                    platform_version.protocol_version,
                    feature_version,
                ),
            ));
        }
        match version {
            RequestVersion::V0(request_v0) => {
                let result = self.query_shielded_encrypted_notes_v0(
                    request_v0,
                    platform_state,
                    platform_version,
                )?;

                Ok(result.map(|response_v0| GetShieldedEncryptedNotesResponse {
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
    use dapi_grpc::platform::v0::get_shielded_encrypted_notes_request::GetShieldedEncryptedNotesRequestV0;
    use dapi_grpc::platform::v0::get_shielded_encrypted_notes_response::get_shielded_encrypted_notes_response_v0;
    use dpp::dashcore::Network;

    #[test]
    fn test_query_shielded_encrypted_notes_with_none_version_returns_decoding_error() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetShieldedEncryptedNotesRequest { version: None };

        let result = platform
            .query_shielded_encrypted_notes(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::DecodingError(msg)] if msg.contains("could not decode shielded encrypted notes query")
        ));
    }

    #[test]
    fn test_query_shielded_encrypted_notes_non_aligned_start_index_rejected() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        // Alignment is to the MMR chunk size (`1 << SHIELDED_NOTES_CHUNK_POWER`
        // = 2048). Anything that isn't a multiple of that is rejected.
        let request = GetShieldedEncryptedNotesRequest {
            version: Some(RequestVersion::V0(GetShieldedEncryptedNotesRequestV0 {
                start_index: 1,
                count: 16,
                prove: false,
            })),
        };

        let result = platform
            .query_shielded_encrypted_notes(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)]
                if msg.contains("not chunk-aligned")
        ));
    }

    #[test]
    fn test_query_shielded_encrypted_notes_empty_state_returns_empty_entries() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        // start_index 0 is always chunk-aligned. Fresh state has no notes, so
        // the non-proved branch should loop once, find None, and return empty.
        let request = GetShieldedEncryptedNotesRequest {
            version: Some(RequestVersion::V0(GetShieldedEncryptedNotesRequestV0 {
                start_index: 0,
                count: 8,
                prove: false,
            })),
        };

        let result = platform
            .query_shielded_encrypted_notes(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "expected no errors");
        let response = result.data.expect("expected response data");
        let inner = match response.version {
            Some(ResponseVersion::V0(v)) => v,
            _ => panic!("expected v0 response"),
        };
        match inner.result {
            Some(get_shielded_encrypted_notes_response_v0::Result::EncryptedNotes(notes)) => {
                assert!(
                    notes.entries.is_empty(),
                    "expected no notes in fresh shielded pool"
                );
            }
            other => panic!("expected EncryptedNotes result, got {:?}", other),
        }
        assert!(inner.metadata.is_some(), "expected metadata present");
    }

    #[test]
    fn test_query_shielded_encrypted_notes_count_zero_uses_max() {
        // When count == 0, the limit falls back to the per-query max
        // (`max_query_chunks × MMR_CHUNK_SIZE`). This exercises the
        // "count == 0 || count > max" branch of the limit derivation
        // without needing notes to be stored.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetShieldedEncryptedNotesRequest {
            version: Some(RequestVersion::V0(GetShieldedEncryptedNotesRequestV0 {
                start_index: 0,
                count: 0,
                prove: false,
            })),
        };

        let result = platform
            .query_shielded_encrypted_notes(request, &state, version)
            .expect("expected query to succeed");

        assert!(
            result.errors.is_empty(),
            "count=0 should be accepted (treated as max); errors: {:?}",
            result.errors
        );
        let response = result.data.expect("expected response data");
        let inner = match response.version {
            Some(ResponseVersion::V0(v)) => v,
            _ => panic!("expected v0 response"),
        };
        // Fresh state still returns empty entries for the non-proof branch.
        assert!(matches!(
            inner.result,
            Some(get_shielded_encrypted_notes_response_v0::Result::EncryptedNotes(_))
        ));
    }

    #[test]
    fn test_query_shielded_encrypted_notes_count_exceeds_max_clamped() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let max = version.drive_abci.query.shielded_queries.max_query_chunks as u32
            * (1u32 << drive::drive::shielded::paths::SHIELDED_NOTES_CHUNK_POWER);

        let request = GetShieldedEncryptedNotesRequest {
            version: Some(RequestVersion::V0(GetShieldedEncryptedNotesRequestV0 {
                start_index: 0,
                count: max + 100,
                prove: false,
            })),
        };

        let result = platform
            .query_shielded_encrypted_notes(request, &state, version)
            .expect("expected query to succeed");

        // Over-max count should be silently clamped to max, not errored out.
        assert!(
            result.errors.is_empty(),
            "count > max should be clamped, not rejected; errors: {:?}",
            result.errors
        );
    }
}
