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
use drive::drive::shielded::paths::shielded_credit_pool_encrypted_notes_path_vec;
use drive::grovedb::query_result_type::QueryResultType;
use drive::grovedb::{PathQuery, Query, SizedQuery};
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
        let max_elements = platform_version.drive_abci.query.max_returned_elements as u32;
        let effective = if count == 0 || count > max_elements {
            max_elements
        } else {
            count
        };
        let limit = effective.min(u16::MAX as u32) as u16;

        let query = if start_index == 0 {
            Query::new_range_full()
        } else {
            let mut q = Query::new();
            q.insert_range_from(start_index.to_be_bytes().to_vec()..);
            q
        };

        let path_query = PathQuery {
            path: shielded_credit_pool_encrypted_notes_path_vec(),
            query: SizedQuery {
                query,
                limit: Some(limit),
                offset: None,
            },
        };

        let response = if prove {
            let proof = check_validation_result_with_data!(self.drive.grove_get_proved_path_query(
                &path_query,
                None,
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
            let (results, _) = self.drive.grove_get_raw_path_query(
                &path_query,
                None,
                QueryResultType::QueryKeyElementPairResultType,
                &mut vec![],
                &platform_version.drive,
            )?;

            let entries: Vec<EncryptedNote> = results
                .to_key_elements()
                .into_iter()
                .filter_map(|(_key, element)| {
                    element.into_item_bytes().ok().and_then(|value| {
                        // Value format: cmx (32 bytes) || encrypted_note (remaining bytes)
                        if value.len() > 32 {
                            Some(EncryptedNote {
                                cmx: value[..32].to_vec(),
                                encrypted_note: value[32..].to_vec(),
                            })
                        } else {
                            None
                        }
                    })
                })
                .collect();

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
