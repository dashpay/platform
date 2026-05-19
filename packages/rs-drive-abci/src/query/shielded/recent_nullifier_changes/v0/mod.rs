use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_recent_nullifier_changes_request::GetRecentNullifierChangesRequestV0;
use dapi_grpc::platform::v0::get_recent_nullifier_changes_response::{
    get_recent_nullifier_changes_response_v0, GetRecentNullifierChangesResponseV0,
};
use dapi_grpc::platform::v0::{BlockNullifierChanges, NullifierUpdateEntries};
use dpp::version::PlatformVersion;
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    pub(super) fn query_recent_nullifier_changes_v0(
        &self,
        GetRecentNullifierChangesRequestV0 {
            start_height,
            prove,
        }: GetRecentNullifierChangesRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetRecentNullifierChangesResponseV0>, Error> {
        let limit = Some(100u16);

        let response = if prove {
            let proof = self.drive.prove_recent_nullifier_changes(
                start_height,
                limit,
                None,
                platform_version,
            )?;

            let (grovedb_used, proof) =
                self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)?;

            GetRecentNullifierChangesResponseV0 {
                result: Some(get_recent_nullifier_changes_response_v0::Result::Proof(
                    proof,
                )),
                metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
            }
        } else {
            let nullifier_changes = self.drive.fetch_recent_nullifier_changes(
                start_height,
                limit,
                None,
                platform_version,
            )?;

            let block_changes: Vec<BlockNullifierChanges> = nullifier_changes
                .into_iter()
                .map(|change| BlockNullifierChanges {
                    block_height: change.block_height,
                    nullifiers: change.nullifiers.iter().map(|n| n.to_vec()).collect(),
                })
                .collect();

            GetRecentNullifierChangesResponseV0 {
                result: Some(
                    get_recent_nullifier_changes_response_v0::Result::NullifierUpdateEntries(
                        NullifierUpdateEntries { block_changes },
                    ),
                ),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
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
        // An empty platform has no recent nullifier changes, so the non-prove
        // branch must return an empty NullifierUpdateEntries and populated metadata.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetRecentNullifierChangesRequestV0 {
            start_height: 0,
            prove: false,
        };

        let result = platform
            .query_recent_nullifier_changes_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        let response = result.data.expect("expected response data");
        match response.result {
            Some(get_recent_nullifier_changes_response_v0::Result::NullifierUpdateEntries(
                entries,
            )) => {
                assert!(
                    entries.block_changes.is_empty(),
                    "expected no block changes on a fresh platform"
                );
            }
            other => panic!("expected NullifierUpdateEntries, got {:?}", other),
        }
        assert!(response.metadata.is_some());
    }

    #[test]
    fn empty_state_prove_returns_proof_bytes() {
        // Prove branch on empty state must yield a non-empty absence proof and
        // populated metadata.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetRecentNullifierChangesRequestV0 {
            start_height: 0,
            prove: true,
        };

        let result = platform
            .query_recent_nullifier_changes_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        let response = result.data.expect("expected response data");
        match response.result {
            Some(get_recent_nullifier_changes_response_v0::Result::Proof(proof)) => {
                assert!(
                    !proof.grovedb_proof.is_empty(),
                    "expected proof bytes for empty recent-changes pool"
                );
            }
            other => panic!("expected Proof result, got {:?}", other),
        }
        assert!(response.metadata.is_some());
    }

    #[test]
    fn large_start_height_returns_empty_entries() {
        // A start height beyond anything stored should return empty entries
        // without erroring – exercises the range-from-start-height branch.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetRecentNullifierChangesRequestV0 {
            start_height: u64::MAX - 1,
            prove: false,
        };

        let result = platform
            .query_recent_nullifier_changes_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        let response = result.data.expect("expected response data");
        match response.result {
            Some(get_recent_nullifier_changes_response_v0::Result::NullifierUpdateEntries(
                entries,
            )) => {
                assert!(entries.block_changes.is_empty());
            }
            other => panic!("expected NullifierUpdateEntries, got {:?}", other),
        }
    }
}
