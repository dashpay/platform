use crate::drive::Drive;
use dpp::data_contract::DataContract;
use grovedb::{GroveDb, PathQuery, SizedQuery};

use crate::error::Error;

use crate::verify::RootHash;

use crate::drive::votes::paths::vote_contested_resource_identity_votes_tree_path_for_identity_vec;
use crate::drive::votes::storage_form::contested_document_resource_reference_storage_form::ContestedDocumentResourceVoteReferenceStorageForm;
use crate::drive::votes::storage_form::contested_document_resource_storage_form::ContestedDocumentResourceVoteStorageForm;
use crate::drive::votes::tree_path_storage_form::TreePathStorageForm;
use crate::error::drive::DriveError;
use crate::error::proof::ProofError;
use crate::query::Query;
use dpp::voting::votes::Vote;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Verifies the masternode vote.
    ///
    /// This function checks the authenticity of a masternode vote by verifying the provided proof.
    /// It can also verify a subset of a larger proof, if specified.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof of authentication from the user.
    /// - `masternode_pro_tx_hash`: A 32-byte array representing the ProTxHash of the masternode.
    /// - `vote`: A reference to the `Vote` struct containing the vote details.
    /// - `data_contract`: A reference to the `DataContract` associated with the vote.
    /// - `verify_subset_of_proof`: A boolean indicating whether to verify a subset of a larger proof.
    /// - `platform_version`: A reference to the `PlatformVersion` struct representing the platform version.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a tuple of `RootHash` and an `Option<Vote>`. The `RootHash` represents
    /// the root hash of GroveDB, and the `Option<Vote>` contains the verified vote if it exists.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof of authentication is not valid.
    /// - The identity ID does not correspond to a valid balance.
    /// - The proved key value is not for the correct path or key in balances.
    /// - More than one balance is found.
    pub(super) fn verify_masternode_vote_v0(
        proof: &[u8],
        masternode_pro_tx_hash: [u8; 32],
        vote: &Vote,
        data_contract: &DataContract,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<Vote>), Error> {
        // First we should get the overall document_type_path
        let path = vote_contested_resource_identity_votes_tree_path_for_identity_vec(
            &masternode_pro_tx_hash,
        );

        let vote_id = vote.vote_poll_unique_id()?;

        let mut query = Query::new();
        query.insert_key(vote_id.to_vec());

        let path_query = PathQuery::new(path, SizedQuery::new(query, Some(1), None));

        let (root_hash, mut proved_key_values) = if verify_subset_of_proof {
            GroveDb::verify_subset_query_with_absence_proof(
                proof,
                &path_query,
                &platform_version.drive.grove_version,
            )?
        } else {
            GroveDb::verify_query_with_absence_proof(
                proof,
                &path_query,
                &platform_version.drive.grove_version,
            )?
        };
        if proved_key_values.len() == 1 {
            let (path, key, maybe_element) = proved_key_values.remove(0);
            let maybe_vote = maybe_element
                .map(|element| {
                    let serialized_reference = element.into_item_bytes()?;
                    let bincode_config = bincode::config::standard()
                        .with_big_endian()
                        .with_no_limit();
                    let reference_storage_form: ContestedDocumentResourceVoteReferenceStorageForm =
                        bincode::decode_from_slice(&serialized_reference, bincode_config)
                            .map_err(|e| {
                                Error::Drive(DriveError::CorruptedSerialization(format!(
                                    "serialization of reference {} is corrupted: {}",
                                    hex::encode(serialized_reference),
                                    e
                                )))
                            })?
                            .0;
                    let absolute_path = reference_storage_form
                        .reference_path_type
                        .absolute_path(path.as_slice(), Some(key.as_slice()))?;
                    let vote_storage_form =
                        ContestedDocumentResourceVoteStorageForm::try_from_tree_path(
                            absolute_path,
                        )?;
                    let resource_vote =
                        vote_storage_form.resolve_with_contract(data_contract, platform_version)?;
                    let proved_vote = resource_vote.into();
                    if &proved_vote != vote {
                        Err(Error::Proof(ProofError::IncorrectProof(format!(
                            "returned vote {:?} does not match the vote that was sent {:?}",
                            &proved_vote, vote
                        ))))
                    } else {
                        Ok::<Vote, Error>(proved_vote)
                    }
                })
                .transpose()?;
            Ok((root_hash, maybe_vote))
        } else {
            Err(Error::Proof(ProofError::TooManyElements(
                "expected one masternode vote",
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::prelude::Identifier;
    use dpp::tests::json_document::json_document_to_contract;
    use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
    use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
    use dpp::voting::vote_polls::VotePoll;
    use dpp::voting::votes::resource_vote::v0::ResourceVoteV0;
    use dpp::voting::votes::resource_vote::ResourceVote;

    #[test]
    fn should_prove_and_verify_absent_masternode_vote() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let data_contract = json_document_to_contract(
            "tests/supporting_files/contract/dpns/dpns-contract.json",
            false,
            platform_version,
        )
        .expect("expected to create a data contract");

        let masternode_pro_tx_hash = [1u8; 32];
        let contender_id = Identifier::from([2u8; 32]);

        let vote = Vote::ResourceVote(ResourceVote::V0(ResourceVoteV0 {
            vote_poll: VotePoll::ContestedDocumentResourceVotePoll(
                ContestedDocumentResourceVotePoll {
                    contract_id: data_contract.id(),
                    document_type_name: "domain".to_string(),
                    index_name: "parentNameAndLabel".to_string(),
                    index_values: vec![dpp::platform_value::Value::Text("dash".to_string())],
                },
            ),
            resource_vote_choice: ResourceVoteChoice::TowardsIdentity(contender_id),
        }));

        // Build the path query as the verify function would
        let path = vote_contested_resource_identity_votes_tree_path_for_identity_vec(
            &masternode_pro_tx_hash,
        );
        let vote_id = vote
            .vote_poll_unique_id()
            .expect("expected vote poll unique id");
        let mut query = Query::new();
        query.insert_key(vote_id.to_vec());
        let path_query = PathQuery::new(path, SizedQuery::new(query, Some(1), None));

        // Generate a proof using the path query
        let proof = drive
            .grove_get_proved_path_query(&path_query, None, &mut vec![], &platform_version.drive)
            .expect("expected to get proof");

        let (_, maybe_vote) = Drive::verify_masternode_vote(
            proof.as_slice(),
            masternode_pro_tx_hash,
            &vote,
            &data_contract,
            false,
            platform_version,
        )
        .expect("expected proof verification to succeed");

        assert_eq!(maybe_vote, None);
    }
}
