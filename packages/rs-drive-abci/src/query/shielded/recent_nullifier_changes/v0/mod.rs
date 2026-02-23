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
                .map(|(block_height, nullifiers)| BlockNullifierChanges {
                    block_height,
                    nullifiers: nullifiers.into_iter().map(|n| n.to_vec()).collect(),
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
