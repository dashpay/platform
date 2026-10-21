use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::query::IdentityBasedVoteDriveQuery;
use crate::verify::bounded_decode::decode_yes_no_vote_reference;
use crate::verify::RootHash;
use dpp::identifier::Identifier;
use dpp::voting::vote_polls::VotePoll;
use dpp::voting::votes::yes_no_vote::accessors::v0::YesNoVoteGettersV0;
use dpp::voting::votes::yes_no_vote::v0::YesNoVoteV0;
use dpp::voting::votes::yes_no_vote::YesNoVote;
use grovedb::GroveDb;
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
        // The prover's own query, so where a masternode's yes/no vote lives is written once.
        let path_query = IdentityBasedVoteDriveQuery {
            identity_id: Identifier::new(masternode_pro_tx_hash),
            vote_poll: VotePoll::YesNoVotePoll(vote.vote_poll().clone()),
        }
        .construct_path_query()?;
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
                let storage_form = decode_yes_no_vote_reference(&bytes)?;
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
