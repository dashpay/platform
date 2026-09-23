use crate::drive::votes::paths::vote_decisions_identity_votes_tree_path_for_identity_vec;
use crate::drive::votes::storage_form::yes_no_vote_reference_storage_form::YesNoVoteReferenceStorageForm;
use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::query::Query;
use crate::verify::RootHash;
use dpp::voting::votes::yes_no_vote::accessors::v0::YesNoVoteGettersV0;
use dpp::voting::votes::yes_no_vote::v0::YesNoVoteV0;
use dpp::voting::votes::yes_no_vote::YesNoVote;
use grovedb::{GroveDb, PathQuery, SizedQuery};
use platform_version::version::PlatformVersion;

impl Drive {
    #[inline(always)]
    pub(super) fn verify_masternode_yes_no_vote_v0(
        proof: &[u8],
        masternode_pro_tx_hash: [u8; 32],
        vote: &YesNoVote,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<YesNoVote>), Error> {
        let path =
            vote_decisions_identity_votes_tree_path_for_identity_vec(&masternode_pro_tx_hash);
        let vote_poll_id = vote.vote_poll().unique_id()?;
        let mut query = Query::new();
        query.insert_key(vote_poll_id.to_vec());
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
        if proved_key_values.len() != 1 {
            return Err(Error::Proof(ProofError::TooManyElements(
                "expected one masternode yes/no vote",
            )));
        }
        let (_, _, maybe_element) = proved_key_values.remove(0);
        let maybe_vote = maybe_element
            .map(|element| {
                let bytes = element.into_item_bytes()?;
                let storage_form =
                    YesNoVoteReferenceStorageForm::deserialize(&bytes).map_err(|e| {
                        Error::Proof(ProofError::CorruptedProof(format!(
                            "the proved yes/no vote could not be decoded: {}",
                            e
                        )))
                    })?;
                let proved_vote = YesNoVote::V0(YesNoVoteV0 {
                    vote_poll: vote.vote_poll().clone(),
                    vote_choice: storage_form.vote_choice,
                });
                if &proved_vote != vote {
                    Err(Error::Proof(ProofError::IncorrectProof(format!(
                        "returned vote {:?} does not match the vote that was sent {:?}",
                        proved_vote, vote
                    ))))
                } else {
                    Ok::<YesNoVote, Error>(proved_vote)
                }
            })
            .transpose()?;
        Ok((root_hash, maybe_vote))
    }
}
