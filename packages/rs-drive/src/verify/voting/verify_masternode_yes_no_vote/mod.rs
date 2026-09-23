mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::version::PlatformVersion;
use dpp::voting::votes::yes_no_vote::YesNoVote;

impl Drive {
    /// Verifies a proof of a masternode's vote on a yes/no poll.
    ///
    /// Returns the root hash and the proved vote, `None` when the proof shows the masternode
    /// has not voted on the poll. A proved vote whose answer differs from `vote` is an error.
    pub fn verify_masternode_yes_no_vote(
        proof: &[u8],
        masternode_pro_tx_hash: [u8; 32],
        vote: &YesNoVote,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<YesNoVote>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .voting
            .verify_masternode_yes_no_vote
        {
            0 => Drive::verify_masternode_yes_no_vote_v0(
                proof,
                masternode_pro_tx_hash,
                vote,
                verify_subset_of_proof,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_masternode_yes_no_vote".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;
    use crate::drive::votes::resolved::votes::resolved_yes_no_vote::ResolvedYesNoVote;
    use crate::error::proof::ProofError;
    use crate::query::IdentityBasedVoteDriveQuery;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::platform_value::{BinaryData, Identifier};
    use dpp::voting::vote_choices::yes_no_abstain_vote_choice::YesNoAbstainVoteChoice;
    use dpp::voting::vote_polls::yes_no_vote_poll::YesNoVotePoll;
    use dpp::voting::vote_polls::VotePoll;
    use dpp::voting::votes::yes_no_vote::v0::YesNoVoteV0;

    fn poll() -> YesNoVotePoll {
        YesNoVotePoll {
            resource_path: vec![BinaryData::new(vec![0xc1; 32])],
            supermajority_numerator: 2,
            supermajority_denominator: 3,
            minimum_voting_power: 400,
        }
    }

    fn vote(vote_choice: YesNoAbstainVoteChoice) -> YesNoVote {
        YesNoVote::V0(YesNoVoteV0 {
            vote_poll: poll(),
            vote_choice,
        })
    }

    fn proof_of_vote(drive: &crate::drive::Drive, voter: [u8; 32]) -> Vec<u8> {
        let platform_version = PlatformVersion::latest();
        let mut path_query = IdentityBasedVoteDriveQuery {
            identity_id: Identifier::new(voter),
            vote_poll: VotePoll::YesNoVotePoll(poll()),
        }
        .construct_path_query()
        .expect("path query");
        path_query.query.limit = None;
        drive
            .grove_get_proved_path_query(&path_query, None, &mut vec![], &platform_version.drive)
            .expect("proof")
    }

    #[test]
    fn should_prove_a_vote_and_its_absence() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        drive
            .open_yes_no_vote_poll(
                &poll(),
                1_000,
                &BlockInfo::default(),
                None,
                platform_version,
            )
            .expect("open");
        let voter = [7u8; 32];
        drive
            .register_yes_no_identity_vote(
                voter,
                1,
                ResolvedYesNoVote {
                    vote_poll: poll(),
                    vote_choice: YesNoAbstainVoteChoice::No,
                    previous_vote_choice_to_remove: None,
                },
                &BlockInfo::default(),
                None,
                platform_version,
            )
            .expect("register");

        let proof = proof_of_vote(&drive, voter);
        let (_, proved) = Drive::verify_masternode_yes_no_vote(
            &proof,
            voter,
            &vote(YesNoAbstainVoteChoice::No),
            false,
            platform_version,
        )
        .expect("verify");
        assert_eq!(proved, Some(vote(YesNoAbstainVoteChoice::No)));

        // The proof shows the answer; claiming another one is caught.
        assert!(matches!(
            Drive::verify_masternode_yes_no_vote(
                &proof,
                voter,
                &vote(YesNoAbstainVoteChoice::Yes),
                false,
                platform_version,
            ),
            Err(Error::Proof(ProofError::IncorrectProof(_)))
        ));

        // A masternode that has not voted proves absent.
        let other = [8u8; 32];
        let proof = proof_of_vote(&drive, other);
        let (_, proved) = Drive::verify_masternode_yes_no_vote(
            &proof,
            other,
            &vote(YesNoAbstainVoteChoice::No),
            false,
            platform_version,
        )
        .expect("verify");
        assert_eq!(proved, None);
    }
}
