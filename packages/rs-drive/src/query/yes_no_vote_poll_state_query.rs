#[cfg(feature = "server")]
use crate::drive::votes::paths::vote_decisions_active_polls_tree_path;
use crate::drive::votes::paths::{YesNoVotePollPaths, YES_NO_VOTE_POLL_STORED_INFO_KEY};
use crate::drive::votes::YesNoAbstainVoteChoiceToKeyTrait;
#[cfg(feature = "server")]
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
#[cfg(feature = "server")]
use crate::fees::op::LowLevelDriveOperation;
use crate::query::Query;
#[cfg(feature = "server")]
use crate::util::grove_operations::DirectQueryType;
use dpp::serialization::PlatformDeserializableUntrusted;
use dpp::voting::vote_choices::yes_no_abstain_vote_choice::YesNoAbstainVoteChoice;
use dpp::voting::vote_info_storage::yes_no_vote_poll_stored_info::YesNoVotePollStoredInfo;
use dpp::voting::vote_polls::yes_no_vote_poll::{VotingPower, YesNoVotePoll};
#[cfg(feature = "server")]
use grovedb::query_result_type::QueryResultType;
#[cfg(feature = "server")]
use grovedb::TransactionArg;
use grovedb::{Element, PathQuery, SizedQuery};
#[cfg(feature = "server")]
use platform_version::version::PlatformVersion;

/// The state of one yes/no vote poll: its stored info and its tallies.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct YesNoVotePollState {
    /// The poll's status and, once finished, its result. `None` when the poll was never opened.
    pub stored_info: Option<YesNoVotePollStoredInfo>,
    /// The yes voting power cast so far. Zero once the poll is finished and its votes removed.
    pub yes_voting_power: VotingPower,
    /// The no voting power cast so far. Zero once the poll is finished and its votes removed.
    pub no_voting_power: VotingPower,
    /// The abstaining voting power cast so far. Zero once the poll is finished and its votes
    /// removed.
    pub abstain_voting_power: VotingPower,
}

impl YesNoVotePollState {
    /// The voting power cast for a choice.
    pub fn voting_power(&self, vote_choice: YesNoAbstainVoteChoice) -> VotingPower {
        match vote_choice {
            YesNoAbstainVoteChoice::Yes => self.yes_voting_power,
            YesNoAbstainVoteChoice::No => self.no_voting_power,
            YesNoAbstainVoteChoice::Abstain => self.abstain_voting_power,
        }
    }
}

/// The query for the state of a yes/no vote poll: its stored info item and its three vote sum
/// trees, whose sums are the tallies.
#[derive(Debug, PartialEq, Clone)]
pub struct YesNoVotePollStateDriveQuery {
    /// The poll asked about
    pub vote_poll: YesNoVotePoll,
}

impl YesNoVotePollStateDriveQuery {
    /// The path query: the four keys of the poll's tree.
    pub fn construct_path_query(&self) -> Result<PathQuery, Error> {
        let path = self.vote_poll.poll_path_vec()?;
        let mut query = Query::new_with_direction(true);
        query.insert_key(vec![YES_NO_VOTE_POLL_STORED_INFO_KEY]);
        for vote_choice in YesNoAbstainVoteChoice::ALL {
            query.insert_key(vec![vote_choice.to_tree_key()]);
        }
        // Four keys at most; absence proofs need the limit stated.
        Ok(PathQuery::new(
            path,
            SizedQuery::new(
                query,
                Some(1 + YesNoAbstainVoteChoice::ALL.len() as u16),
                None,
            ),
        ))
    }

    /// Reads the state out of the elements of the poll's tree, by key. Keys that are absent
    /// leave their part of the state empty: a poll that was never opened has no stored info,
    /// and a finished poll has no vote trees.
    pub(crate) fn state_from_key_elements(
        key_elements: impl IntoIterator<Item = (Vec<u8>, Element)>,
    ) -> Result<YesNoVotePollState, Error> {
        let mut state = YesNoVotePollState::default();
        for (key, element) in key_elements {
            let [key_byte] = key.as_slice() else {
                return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                    "unexpected key {} in yes/no vote poll tree",
                    hex::encode(&key)
                ))));
            };
            if *key_byte == YES_NO_VOTE_POLL_STORED_INFO_KEY {
                let bytes = element.into_item_bytes()?;
                state.stored_info = Some(
                    YesNoVotePollStoredInfo::deserialize_from_bytes_untrusted(&bytes)?,
                );
                continue;
            }
            let Some(vote_choice) = YesNoAbstainVoteChoice::from_tree_key(*key_byte) else {
                return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                    "unexpected key {} in yes/no vote poll tree",
                    hex::encode(&key)
                ))));
            };
            let Element::SumTree(_, sum, _) = element else {
                return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                    "the {} votes of a yes/no vote poll must be a sum tree",
                    vote_choice
                ))));
            };
            let voting_power: VotingPower = sum.try_into().map_err(|_| {
                Error::Drive(DriveError::CorruptedDriveState(format!(
                    "the {} voting power of a yes/no vote poll must be between 0 and u32::MAX, got {}",
                    vote_choice, sum
                )))
            })?;
            match vote_choice {
                YesNoAbstainVoteChoice::Yes => state.yes_voting_power = voting_power,
                YesNoAbstainVoteChoice::No => state.no_voting_power = voting_power,
                YesNoAbstainVoteChoice::Abstain => state.abstain_voting_power = voting_power,
            }
        }
        Ok(state)
    }

    /// Executes the query and returns the proof.
    #[cfg(feature = "server")]
    pub fn execute_with_proof(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let path_query = self.construct_path_query()?;
        drive.grove_get_proved_path_query(
            &path_query,
            transaction,
            drive_operations,
            &platform_version.drive,
        )
    }

    /// Executes the query and returns the state.
    #[cfg(feature = "server")]
    pub fn execute_no_proof(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<YesNoVotePollState, Error> {
        // A poll that was never opened has no tree; the proof path reports the same as an
        // empty state, so does this one.
        let poll_id = self.vote_poll.unique_id()?;
        let exists = drive.grove_has_raw(
            (&vote_decisions_active_polls_tree_path()).into(),
            poll_id.as_bytes(),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            drive_operations,
            &platform_version.drive,
        )?;
        if !exists {
            return Ok(YesNoVotePollState::default());
        }
        let path_query = self.construct_path_query()?;
        let key_elements = drive
            .grove_get_raw_path_query(
                &path_query,
                transaction,
                QueryResultType::QueryKeyElementPairResultType,
                drive_operations,
                &platform_version.drive,
            )?
            .0
            .to_key_elements();
        Self::state_from_key_elements(key_elements)
    }
}

#[cfg(all(test, feature = "server", feature = "verify"))]
mod tests {
    use super::*;
    use crate::drive::votes::resolved::votes::resolved_yes_no_vote::ResolvedYesNoVote;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::platform_value::BinaryData;
    use dpp::voting::vote_info_storage::yes_no_vote_poll_stored_info::YesNoVotePollStatus;

    fn poll() -> YesNoVotePoll {
        YesNoVotePoll {
            resource_path: vec![BinaryData::new(vec![0xc1; 32])],
            supermajority_numerator: 2,
            supermajority_denominator: 3,
            minimum_voting_power: 400,
        }
    }

    #[test]
    fn should_prove_and_verify_the_state_of_a_poll_with_votes() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let vote_poll = poll();
        drive
            .open_yes_no_vote_poll(
                &vote_poll,
                1_000,
                &BlockInfo::default(),
                None,
                platform_version,
            )
            .expect("expected to open the poll");
        for (voter, strength, vote_choice) in [
            ([1u8; 32], 4, YesNoAbstainVoteChoice::Yes),
            ([2u8; 32], 1, YesNoAbstainVoteChoice::Yes),
            ([3u8; 32], 1, YesNoAbstainVoteChoice::No),
        ] {
            drive
                .register_yes_no_identity_vote(
                    voter,
                    strength,
                    ResolvedYesNoVote {
                        vote_poll: vote_poll.clone(),
                        vote_choice,
                        previous_vote_choice_to_remove: None,
                    },
                    &BlockInfo::default(),
                    None,
                    platform_version,
                )
                .expect("expected to register the vote");
        }
        let query = YesNoVotePollStateDriveQuery { vote_poll };
        let state = query
            .execute_no_proof(&drive, None, &mut vec![], platform_version)
            .expect("expected the state");
        assert_eq!(
            (
                state.yes_voting_power,
                state.no_voting_power,
                state.abstain_voting_power
            ),
            (5, 1, 0)
        );
        assert!(matches!(
            state.stored_info.as_ref().expect("stored info").status(),
            YesNoVotePollStatus::Started(_)
        ));

        let proof = query
            .execute_with_proof(&drive, None, &mut vec![], platform_version)
            .expect("expected a proof");
        let (root_hash, proved_state) = query
            .verify_yes_no_vote_poll_state_proof(&proof, platform_version)
            .expect("expected the proof to verify");
        assert_eq!(
            root_hash,
            drive
                .grove
                .root_hash(None, &platform_version.drive.grove_version)
                .unwrap()
                .expect("root hash")
        );
        assert_eq!(proved_state, state);
    }

    #[test]
    fn should_prove_and_verify_the_absence_of_a_poll() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let query = YesNoVotePollStateDriveQuery { vote_poll: poll() };
        let proof = query
            .execute_with_proof(&drive, None, &mut vec![], platform_version)
            .expect("expected a proof");
        let (_, proved_state) = query
            .verify_yes_no_vote_poll_state_proof(&proof, platform_version)
            .expect("expected the proof to verify");
        assert_eq!(proved_state, YesNoVotePollState::default());
        let state = query
            .execute_no_proof(&drive, None, &mut vec![], platform_version)
            .expect("expected the state of an absent poll");
        assert_eq!(state, YesNoVotePollState::default());
    }
}
