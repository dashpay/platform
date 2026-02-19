use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_shielded_encrypted_notes_request::GetShieldedEncryptedNotesRequestV0;
use dapi_grpc::platform::v0::get_shielded_encrypted_notes_response::get_shielded_encrypted_notes_response_v0::{
    EncryptedNote, EncryptedNotes,
};
use dapi_grpc::platform::v0::get_shielded_encrypted_notes_response::{
    get_shielded_encrypted_notes_response_v0, GetShieldedEncryptedNotesResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::shielded::paths::{
    shielded_credit_pool_path, shielded_credit_pool_path_vec, SHIELDED_NOTES_KEY,
};
use drive::grovedb::{PathQuery, Query, QueryItem, SizedQuery, SubqueryBranch};
use drive::grovedb_path::SubtreePath;
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    pub(super) fn query_shielded_encrypted_notes_v0(
        &self,
        GetShieldedEncryptedNotesRequestV0 {
            start_index,
            count,
            prove,
        }: GetShieldedEncryptedNotesRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetShieldedEncryptedNotesResponseV0>, Error> {
        let max_notes = platform_version
            .drive_abci
            .query
            .shielded_queries
            .max_encrypted_notes_per_query as u32;

        // start_index must be chunk-aligned (multiple of max_notes) so each
        // query touches exactly one MMR chunk or the buffer.
        let chunk_size = max_notes as u64;
        if start_index % chunk_size != 0 {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::InvalidArgument(format!(
                    "start_index {} is not chunk-aligned; must be a multiple of {}",
                    start_index, chunk_size
                )),
            ));
        }

        let effective = if count == 0 || count > max_notes {
            max_notes
        } else {
            count
        };
        let limit = effective.min(u16::MAX as u32) as u16;

        let response = if prove {
            // V1 proof: PathQuery with subquery targeting positions in the CommitmentTree
            let end_index = start_index + limit as u64 - 1;
            let mut inner_query = Query::new();
            inner_query.insert_range_inclusive(
                start_index.to_be_bytes().to_vec()..=end_index.to_be_bytes().to_vec(),
            );

            let path_query = PathQuery {
                path: shielded_credit_pool_path_vec(),
                query: SizedQuery {
                    query: Query {
                        items: vec![QueryItem::Key(vec![SHIELDED_NOTES_KEY])],
                        default_subquery_branch: SubqueryBranch {
                            subquery_path: None,
                            subquery: Some(inner_query.into()),
                        },
                        left_to_right: true,
                        conditional_subquery_branches: None,
                        add_parent_tree_on_subquery: false,
                    },
                    limit: None,
                    offset: None,
                },
            };

            let proof =
                check_validation_result_with_data!(self.drive.grove_get_proved_path_query_v1(
                    &path_query,
                    &mut vec![],
                    &platform_version.drive,
                ));

            let (grovedb_used, proof) =
                self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)?;

            GetShieldedEncryptedNotesResponseV0 {
                result: Some(get_shielded_encrypted_notes_response_v0::Result::Proof(
                    proof,
                )),
                metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
            }
        } else {
            // Non-proved: loop over commitment_tree_get_value for each position
            let pool_path = shielded_credit_pool_path();
            let pool_subtree: SubtreePath<&[u8]> = (&pool_path).into();
            let notes_key: &[u8] = &[SHIELDED_NOTES_KEY];

            let mut entries = Vec::with_capacity(limit as usize);
            for pos in start_index..(start_index + limit as u64) {
                let maybe_value = self
                    .drive
                    .grove
                    .commitment_tree_get_value(
                        pool_subtree.clone(),
                        notes_key,
                        pos,
                        None,
                        &platform_version.drive.grove_version,
                    )
                    .unwrap()
                    .map_err(|e| Error::Drive(drive::error::Error::GroveDB(Box::new(e))))?;

                match maybe_value {
                    // Stored value = cmx (32) || nullifier (32) || encrypted_note (rest)
                    Some(value) if value.len() > 64 => {
                        entries.push(EncryptedNote {
                            cmx: value[..32].to_vec(),
                            nullifier: value[32..64].to_vec(),
                            encrypted_note: value[64..].to_vec(),
                        });
                    }
                    _ => break, // past end of tree
                }
            }

            GetShieldedEncryptedNotesResponseV0 {
                result: Some(
                    get_shielded_encrypted_notes_response_v0::Result::EncryptedNotes(
                        EncryptedNotes { entries },
                    ),
                ),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
