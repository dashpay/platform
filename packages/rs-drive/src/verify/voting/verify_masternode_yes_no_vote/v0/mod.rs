use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::query::masternode_yes_no_vote_query::MasternodeYesNoVoteDriveQuery;
use crate::verify::RootHash;
use dpp::identifier::Identifier;
use dpp::voting::votes::yes_no_vote::accessors::v0::YesNoVoteGettersV0;
use dpp::voting::votes::yes_no_vote::v0::YesNoVoteV0;
use dpp::voting::votes::yes_no_vote::YesNoVote;
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
        // The masternode's answer by the poll's id, read as the query verifier reads it.
        let query = MasternodeYesNoVoteDriveQuery {
            pro_tx_hash: Identifier::new(masternode_pro_tx_hash),
            vote_poll_id: vote.vote_poll().unique_id(platform_version)?,
        };
        let (root_hash, maybe_answer) = query.verify_identity_yes_no_vote_v0(
            proof,
            verify_subset_of_proof,
            platform_version,
        )?;
        let maybe_vote = maybe_answer
            .map(|answer| {
                let proved_vote = YesNoVote::V0(YesNoVoteV0 {
                    vote_poll: vote.vote_poll().clone(),
                    vote_choice: answer.vote_choice,
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
