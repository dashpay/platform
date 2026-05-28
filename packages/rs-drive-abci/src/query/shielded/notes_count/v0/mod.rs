use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_shielded_notes_count_request::GetShieldedNotesCountRequestV0;
use dapi_grpc::platform::v0::get_shielded_notes_count_response::GetShieldedNotesCountResponseV0;
use dpp::version::PlatformVersion;

impl<C> Platform<C> {
    /// Counts the total number of notes in the shielded credit
    /// pool's CommitmentTree.
    ///
    /// Delegates to [`drive::drive::Drive::shielded_pool_notes_count`],
    /// which reads the leaf count off the tree without walking it.
    /// Unproved — the count is derived tree metadata, not a stored
    /// key, so there is no companion proof variant.
    pub(super) fn query_shielded_notes_count_v0(
        &self,
        GetShieldedNotesCountRequestV0 {}: GetShieldedNotesCountRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetShieldedNotesCountResponseV0>, Error> {
        let total_notes_count =
            self.drive
                .shielded_pool_notes_count(None, &mut vec![], platform_version)?;

        let response = GetShieldedNotesCountResponseV0 {
            total_notes_count,
            metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
