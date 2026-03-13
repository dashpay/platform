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
