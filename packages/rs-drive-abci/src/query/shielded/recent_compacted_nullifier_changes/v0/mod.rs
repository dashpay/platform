use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_recent_compacted_nullifier_changes_request::GetRecentCompactedNullifierChangesRequestV0;
use dapi_grpc::platform::v0::get_recent_compacted_nullifier_changes_response::{
    get_recent_compacted_nullifier_changes_response_v0,
    GetRecentCompactedNullifierChangesResponseV0,
};
use dapi_grpc::platform::v0::{CompactedBlockNullifierChanges, CompactedNullifierUpdateEntries};
use dpp::version::PlatformVersion;
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    pub(super) fn query_recent_compacted_nullifier_changes_v0(
        &self,
        GetRecentCompactedNullifierChangesRequestV0 {
            start_block_height,
            prove,
        }: GetRecentCompactedNullifierChangesRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetRecentCompactedNullifierChangesResponseV0>, Error> {
        let limit = Some(25u16);

        let response = if prove {
            let proof = self.drive.prove_compacted_nullifier_changes(
                start_block_height,
                limit,
                None,
                platform_version,
            )?;

            let (grovedb_used, proof) =
                self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)?;

            GetRecentCompactedNullifierChangesResponseV0 {
                result: Some(
                    get_recent_compacted_nullifier_changes_response_v0::Result::Proof(proof),
                ),
                metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
            }
        } else {
            let compacted_nullifier_changes = self.drive.fetch_compacted_nullifier_changes(
                start_block_height,
                limit,
                None,
                platform_version,
            )?;

            let compacted_block_changes: Vec<CompactedBlockNullifierChanges> =
                compacted_nullifier_changes
                    .into_iter()
                    .map(|change| CompactedBlockNullifierChanges {
                        start_block_height: change.start_block,
                        end_block_height: change.end_block,
                        nullifiers: change.nullifiers.iter().map(|n| n.to_vec()).collect(),
                    })
                    .collect();

            GetRecentCompactedNullifierChangesResponseV0 {
                result: Some(
                    get_recent_compacted_nullifier_changes_response_v0::Result::CompactedNullifierUpdateEntries(
                        CompactedNullifierUpdateEntries { compacted_block_changes },
                    ),
                ),
                metadata: Some(
                    self.response_metadata_v0(platform_state, CheckpointUsed::Current),
                ),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::setup_platform;
    use dpp::dashcore::Network;

    #[test]
    fn empty_state_non_prove_returns_empty_entries() {
        // With no compactions stored, the fetch returns an empty vector and the
        // handler must wrap it in CompactedNullifierUpdateEntries with an empty
        // list and populated metadata.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetRecentCompactedNullifierChangesRequestV0 {
            start_block_height: 0,
            prove: false,
        };

        let result = platform
            .query_recent_compacted_nullifier_changes_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        let response = result.data.expect("expected response data");
        match response.result {
            Some(
                get_recent_compacted_nullifier_changes_response_v0::Result::CompactedNullifierUpdateEntries(
                    entries,
                ),
            ) => {
                assert!(
                    entries.compacted_block_changes.is_empty(),
                    "expected empty compacted block changes on a fresh platform"
                );
            }
            other => panic!(
                "expected CompactedNullifierUpdateEntries result, got {:?}",
                other
            ),
        }
        assert!(response.metadata.is_some());
    }

    #[test]
    fn empty_state_prove_returns_proof_bytes() {
        // The prove branch should produce non-empty proof bytes even for an
        // empty shielded pool – the proof encodes the absence.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetRecentCompactedNullifierChangesRequestV0 {
            start_block_height: 0,
            prove: true,
        };

        let result = platform
            .query_recent_compacted_nullifier_changes_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        let response = result.data.expect("expected response data");
        match response.result {
            Some(get_recent_compacted_nullifier_changes_response_v0::Result::Proof(proof)) => {
                assert!(
                    !proof.grovedb_proof.is_empty(),
                    "expected proof bytes for empty compacted pool"
                );
            }
            other => panic!("expected Proof result, got {:?}", other),
        }
        assert!(response.metadata.is_some());
    }

    #[test]
    fn large_start_block_height_returns_empty_entries() {
        // Querying well beyond any stored block should return an empty list
        // without error. This exercises the "start > everything" branch of the
        // compacted nullifier fetch.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetRecentCompactedNullifierChangesRequestV0 {
            start_block_height: u64::MAX - 1,
            prove: false,
        };

        let result = platform
            .query_recent_compacted_nullifier_changes_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        let response = result.data.expect("expected response data");
        match response.result {
            Some(
                get_recent_compacted_nullifier_changes_response_v0::Result::CompactedNullifierUpdateEntries(
                    entries,
                ),
            ) => {
                assert!(entries.compacted_block_changes.is_empty());
            }
            other => panic!(
                "expected CompactedNullifierUpdateEntries result, got {:?}",
                other
            ),
        }
    }
}
