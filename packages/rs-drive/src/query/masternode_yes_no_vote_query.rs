use crate::drive::votes::paths::vote_decisions_identity_votes_tree_path_for_identity_vec;
#[cfg(feature = "server")]
use crate::drive::Drive;
#[cfg(feature = "server")]
use crate::error::drive::DriveError;
#[cfg(feature = "server")]
use crate::error::Error;
#[cfg(feature = "server")]
use crate::fees::op::LowLevelDriveOperation;
use crate::query::Query;
use dpp::identifier::Identifier;
use dpp::voting::vote_choices::yes_no_abstain_vote_choice::YesNoAbstainVoteChoice;
#[cfg(feature = "server")]
use grovedb::TransactionArg;
use grovedb::{PathQuery, SizedQuery};
#[cfg(feature = "server")]
use platform_version::version::PlatformVersion;

/// A masternode's answer on a yes/no vote poll.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MasternodeYesNoVote {
    /// The masternode's current answer
    pub vote_choice: YesNoAbstainVoteChoice,
    /// How many times the masternode has voted on the poll, its first vote included
    pub times_voted: u16,
}

/// The query for a masternode's answer on a yes/no vote poll: the item keyed by the poll's id
/// under the masternode's tree of yes/no votes. The item is removed when the poll finishes.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MasternodeYesNoVoteDriveQuery {
    /// The masternode's pro tx hash
    pub pro_tx_hash: Identifier,
    /// The id of the poll, as `YesNoVotePoll::unique_id` derives it
    pub vote_poll_id: Identifier,
}

impl MasternodeYesNoVoteDriveQuery {
    /// The path query: the poll's key under the masternode's tree of yes/no votes.
    pub fn construct_path_query(&self) -> PathQuery {
        let path =
            vote_decisions_identity_votes_tree_path_for_identity_vec(self.pro_tx_hash.as_bytes());
        let mut query = Query::new();
        query.insert_key(self.vote_poll_id.to_vec());
        PathQuery::new(path, SizedQuery::new(query, Some(1), None))
    }

    /// Proves the masternode's answer, or its absence, by the method version the platform
    /// version selects (`vote.yes_no.prove_identity_yes_no_vote`, not active before protocol
    /// version 15).
    ///
    /// # Parameters
    /// - `drive`: the Drive to read.
    /// - `transaction`: the transaction to read in.
    /// - `drive_operations`: collects the read's operations.
    /// - `platform_version`: the platform version.
    ///
    /// # Returns
    /// The proof.
    #[cfg(feature = "server")]
    pub fn execute_with_proof(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .vote
            .yes_no
            .prove_identity_yes_no_vote
        {
            Some(0) => drive.grove_get_proved_path_query(
                &self.construct_path_query(),
                transaction,
                drive_operations,
                &platform_version.drive,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_identity_yes_no_vote".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "prove_identity_yes_no_vote".to_string(),
                known_versions: vec![0],
            })),
        }
    }

    /// Reads the masternode's answer, or `None` when it has not voted on the poll (or the poll
    /// finished and its votes were removed), through `Drive::fetch_identity_yes_no_vote`.
    ///
    /// # Parameters
    /// - `drive`: the Drive to read.
    /// - `transaction`: the transaction to read in.
    /// - `drive_operations`: collects the read's operations.
    /// - `platform_version`: the platform version.
    ///
    /// # Returns
    /// The masternode's answer, if any.
    #[cfg(feature = "server")]
    pub fn execute_no_proof(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<MasternodeYesNoVote>, Error> {
        Ok(drive
            .fetch_identity_yes_no_vote(
                self.pro_tx_hash,
                self.vote_poll_id,
                transaction,
                drive_operations,
                platform_version,
            )?
            .map(|(vote_choice, times_voted)| MasternodeYesNoVote {
                vote_choice,
                times_voted,
            }))
    }
}

#[cfg(all(test, feature = "server", feature = "verify"))]
mod tests {
    use super::*;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use crate::util::test_helpers::{open_yes_no_vote_poll_with_votes, test_yes_no_vote_poll};

    #[test]
    fn should_prove_and_verify_a_masternode_answer_and_its_absence() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let vote_poll = test_yes_no_vote_poll();
        let voter = [7u8; 32];
        open_yes_no_vote_poll_with_votes(
            &drive,
            &vote_poll,
            &[(voter, 4, YesNoAbstainVoteChoice::No)],
            platform_version,
        );
        let vote_poll_id = vote_poll.unique_id(platform_version).expect("poll id");
        let root_hash = drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("root hash");

        // The voter's answer, read and proved; a masternode that never voted on any yes/no
        // poll has no tree at all and proves absent.
        for (pro_tx_hash, expected) in [
            (
                voter,
                Some(MasternodeYesNoVote {
                    vote_choice: YesNoAbstainVoteChoice::No,
                    times_voted: 1,
                }),
            ),
            ([8u8; 32], None),
        ] {
            let query = MasternodeYesNoVoteDriveQuery {
                pro_tx_hash: Identifier::new(pro_tx_hash),
                vote_poll_id,
            };
            let read = query
                .execute_no_proof(&drive, None, &mut vec![], platform_version)
                .expect("expected the answer");
            assert_eq!(read, expected);
            let proof = query
                .execute_with_proof(&drive, None, &mut vec![], platform_version)
                .expect("expected a proof");
            let (proved_root_hash, proved) = query
                .verify_identity_yes_no_vote(&proof, false, platform_version)
                .expect("expected the proof to verify");
            assert_eq!(proved_root_hash, root_hash);
            assert_eq!(proved, expected);
        }

        // Another poll's id under the voter's tree proves absent too.
        let other_poll_query = MasternodeYesNoVoteDriveQuery {
            pro_tx_hash: Identifier::new(voter),
            vote_poll_id: Identifier::new([9u8; 32]),
        };
        let proof = other_poll_query
            .execute_with_proof(&drive, None, &mut vec![], platform_version)
            .expect("expected a proof");
        let (_, proved) = other_poll_query
            .verify_identity_yes_no_vote(&proof, false, platform_version)
            .expect("expected the proof to verify");
        assert_eq!(proved, None);
    }

    #[test]
    fn should_refuse_a_proof_of_another_masternode() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let vote_poll = test_yes_no_vote_poll();
        let voter = [7u8; 32];
        open_yes_no_vote_poll_with_votes(
            &drive,
            &vote_poll,
            &[(voter, 1, YesNoAbstainVoteChoice::Yes)],
            platform_version,
        );
        let vote_poll_id = vote_poll.unique_id(platform_version).expect("poll id");
        let proof = MasternodeYesNoVoteDriveQuery {
            pro_tx_hash: Identifier::new(voter),
            vote_poll_id,
        }
        .execute_with_proof(&drive, None, &mut vec![], platform_version)
        .expect("expected a proof");

        // The proof answers for the voter only; read as another masternode's it does not
        // verify.
        assert!(MasternodeYesNoVoteDriveQuery {
            pro_tx_hash: Identifier::new([8u8; 32]),
            vote_poll_id,
        }
        .verify_identity_yes_no_vote(&proof, false, platform_version)
        .is_err());
    }
}
