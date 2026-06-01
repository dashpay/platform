use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_shielded_notes_count_request::GetShieldedNotesCountRequestV0;
use dapi_grpc::platform::v0::get_shielded_notes_count_response::{
    get_shielded_notes_count_response_v0, GetShieldedNotesCountResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::shielded::paths::{shielded_credit_pool_path_vec, SHIELDED_NOTES_KEY};
use drive::grovedb::{PathQuery, Query, SizedQuery};
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    /// Counts the total number of notes in the shielded credit pool's
    /// CommitmentTree, with optional cryptographic proof.
    ///
    /// - Unproved: reads the leaf count off the tree via
    ///   [`drive::drive::Drive::shielded_pool_notes_count`].
    /// - Proved: returns a GroveDB proof of the `CommitmentTree` element
    ///   itself (a single-key query at the pool path, no subquery). The
    ///   count is the element's first field (`total_count`) and its bytes
    ///   are bound into the Merk value hash, so the proof authenticates
    ///   the count against the app/root hash — mirroring
    ///   `query_shielded_pool_state_v0`.
    pub(super) fn query_shielded_notes_count_v0(
        &self,
        GetShieldedNotesCountRequestV0 { prove }: GetShieldedNotesCountRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetShieldedNotesCountResponseV0>, Error> {
        let response = if prove {
            // Single-key query lands on the notes CommitmentTree element
            // (no subquery), so the proof carries the serialized element
            // whose `total_count` is hashed into the root.
            let path_query = PathQuery {
                path: shielded_credit_pool_path_vec(),
                query: SizedQuery {
                    query: Query::new_single_key(vec![SHIELDED_NOTES_KEY]),
                    limit: Some(1),
                    offset: None,
                },
            };

            let proof = check_validation_result_with_data!(self.drive.grove_get_proved_path_query(
                &path_query,
                None,
                &mut vec![],
                &platform_version.drive,
            ));

            let (grovedb_used, proof) =
                self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)?;

            GetShieldedNotesCountResponseV0 {
                result: Some(get_shielded_notes_count_response_v0::Result::Proof(proof)),
                metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
            }
        } else {
            let total_notes_count =
                self.drive
                    .shielded_pool_notes_count(None, &mut vec![], platform_version)?;

            GetShieldedNotesCountResponseV0 {
                result: Some(
                    get_shielded_notes_count_response_v0::Result::TotalNotesCount(
                        total_notes_count,
                    ),
                ),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
