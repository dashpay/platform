use crate::verify::RootHash;
use dpp::identifier::Identifier;
use dpp::platform_value;
use grovedb::GroveDb;

use crate::error::Error;

use crate::query::vote_poll_contestant_votes_query::ResolvedContestedDocumentVotePollVotesDriveQuery;
use dpp::version::PlatformVersion;

impl ResolvedContestedDocumentVotePollVotesDriveQuery<'_> {
    /// Verifies a proof for a collection of documents.
    ///
    /// This function takes a slice of bytes `proof` containing a serialized proof,
    /// verifies it, and returns a tuple consisting of the root hash and a vector of deserialized documents.
    ///
    /// # Arguments
    ///
    /// * `proof` - A byte slice representing the proof to be verified.
    /// * `drive_version` - The current active drive version
    ///
    /// # Returns
    ///
    /// A `Result` containing:
    /// * A tuple with the root hash and a vector of deserialized `Document`s, if the proof is valid.
    /// * An `Error` variant, in case the proof verification fails or deserialization error occurs.
    ///
    /// # Errors
    ///
    /// This function will return an `Error` variant if:
    /// 1. The proof verification fails.
    /// 2. There is a deserialization error when parsing the serialized document(s) into `Document` struct(s).
    #[inline(always)]
    pub(super) fn verify_vote_poll_votes_proof_v0(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<Identifier>), Error> {
        let path_query = self.construct_path_query(platform_version)?;
        let (root_hash, proved_key_values) =
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?;
        let voters = proved_key_values
            .into_iter()
            .map(|(_, voter_id, _)| Identifier::try_from(voter_id))
            .collect::<Result<Vec<Identifier>, platform_value::Error>>()?;

        Ok((root_hash, voters))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed;
    use crate::util::object_size_info::DataContractResolvedInfo;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::tests::json_document::json_document_to_contract;
    use std::sync::Arc;

    #[test]
    fn should_prove_and_verify_empty_vote_poll_votes_proof() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let data_contract = json_document_to_contract(
            "tests/supporting_files/contract/dpns/dpns-contract-contested-unique-index.json",
            false,
            platform_version,
        )
        .expect("expected to create a data contract");

        // Insert the DPNS contract so its paths exist in the store
        drive
            .insert_contract(
                &data_contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert contract");

        let arc_contract = Arc::new(data_contract);
        let contestant_id = Identifier::random();

        let query = ResolvedContestedDocumentVotePollVotesDriveQuery {
            vote_poll: ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed {
                contract: DataContractResolvedInfo::ArcDataContract(arc_contract),
                document_type_name: "domain".to_string(),
                index_name: "parentNameAndLabel".to_string(),
                index_values: vec![
                    dpp::platform_value::Value::Text("dash".to_string()),
                    dpp::platform_value::Value::Text("test".to_string()),
                ],
            },
            contestant_id,
            offset: None,
            limit: Some(10),
            start_at: None,
            order_ascending: true,
        };

        let path_query = query
            .construct_path_query(platform_version)
            .expect("expected to construct path query");

        let proof = drive
            .grove_get_proved_path_query(&path_query, None, &mut vec![], &platform_version.drive)
            .expect("expected to get proof");

        let (_, voters) = query
            .verify_vote_poll_votes_proof(proof.as_slice(), platform_version)
            .expect("expected proof verification to succeed");

        assert!(voters.is_empty());
    }
}
