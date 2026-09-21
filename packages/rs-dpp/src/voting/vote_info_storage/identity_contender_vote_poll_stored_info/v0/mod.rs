use crate::block::block_info::BlockInfo;
use crate::identity::TimestampMillis;
use crate::voting::contender_structs::FinalizedResourceVoteChoicesWithVoterInfo;
use crate::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use crate::voting::vote_info_storage::identity_contender_vote_poll_stored_info::IdentityContenderVotePollStatus;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_value::Identifier;
use std::fmt;

/// How an identity contender vote poll ended.
#[derive(Debug, PartialEq, Eq, Clone, Default, Encode, Decode, DecodeUntrusted)]
pub struct IdentityContenderVotePollResultV0 {
    /// The winning contender. None when the join phase ended with no contender at all.
    pub winner: Option<Identifier>,
    /// Whether a vote phase ran. A single contender wins when the join phase ends, with none.
    pub vote_phase_held: bool,
    /// The block the poll resolved in.
    pub finalization_block: BlockInfo,
    /// Every choice with the masternodes that voted for it and their strength: one entry per
    /// contender, plus abstain, so the tallies can be read back after the votes are gone.
    pub resource_vote_choices: Vec<FinalizedResourceVoteChoicesWithVoterInfo>,
}

impl IdentityContenderVotePollResultV0 {
    /// The tally of a choice from the voters kept in the record.
    pub fn tally_of(&self, choice: &ResourceVoteChoice) -> u32 {
        self.resource_vote_choices
            .iter()
            .filter(|entry| &entry.resource_vote_choice == choice)
            .map(|entry| {
                entry
                    .voters
                    .iter()
                    .map(|(_, strength)| *strength as u32)
                    .sum::<u32>()
            })
            .sum()
    }
}

impl fmt::Display for IdentityContenderVotePollResultV0 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let choices: Vec<String> = self
            .resource_vote_choices
            .iter()
            .map(|choice| choice.to_string())
            .collect();
        write!(
            f,
            "IdentityContenderVotePollResultV0 {{ winner: {}, vote_phase_held: {}, finalization_block: {}, resource_vote_choices: [{}] }}",
            self.winner
                .map(|winner| format!("{}", winner))
                .unwrap_or_else(|| "None".to_string()),
            self.vote_phase_held,
            self.finalization_block,
            choices.join(", ")
        )
    }
}

/// The state of an identity contender vote poll: its phase, when each phase ends, and once it
/// resolved, how.
#[derive(Debug, PartialEq, Eq, Clone, Default, Encode, Decode, DecodeUntrusted)]
pub struct IdentityContenderVotePollStoredInfoV0 {
    /// The block the poll opened in.
    pub start_block: BlockInfo,
    /// When the join phase ends: contenders may join until this time.
    pub join_end_time_ms: TimestampMillis,
    /// When the vote phase ends, if one runs: masternodes may vote from the end of the join
    /// phase until this time.
    pub vote_end_time_ms: TimestampMillis,
    /// The phase the poll is in.
    pub status: IdentityContenderVotePollStatus,
    /// The outcome, once the poll resolved.
    pub result: Option<IdentityContenderVotePollResultV0>,
}

impl fmt::Display for IdentityContenderVotePollStoredInfoV0 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "IdentityContenderVotePollStoredInfoV0 {{ start_block: {}, join_end_time_ms: {}, vote_end_time_ms: {}, status: {}, result: {} }}",
            self.start_block,
            self.join_end_time_ms,
            self.vote_end_time_ms,
            self.status,
            self.result
                .as_ref()
                .map(|result| result.to_string())
                .unwrap_or_else(|| "None".to_string())
        )
    }
}

impl IdentityContenderVotePollStoredInfoV0 {
    /// A poll that just opened, in its join phase.
    pub fn new(
        start_block: BlockInfo,
        join_end_time_ms: TimestampMillis,
        vote_end_time_ms: TimestampMillis,
    ) -> Self {
        Self {
            start_block,
            join_end_time_ms,
            vote_end_time_ms,
            status: IdentityContenderVotePollStatus::Joining,
            result: None,
        }
    }

    /// Moves the poll from its join phase to its vote phase.
    pub fn start_vote_phase(&mut self) -> Result<(), ProtocolError> {
        if self.status != IdentityContenderVotePollStatus::Joining {
            return Err(ProtocolError::CorruptedCodeExecution(format!(
                "trying to start the vote phase of an identity contender vote poll that is {}",
                self.status
            )));
        }
        self.status = IdentityContenderVotePollStatus::Voting;
        Ok(())
    }

    /// Records how the poll ended and marks it resolved. `vote_phase_held` says whether the
    /// masternodes voted, or a single contender won when the join phase ended.
    pub fn resolve(
        &mut self,
        winner: Option<Identifier>,
        resource_vote_choices: Vec<FinalizedResourceVoteChoicesWithVoterInfo>,
        finalization_block: BlockInfo,
    ) -> Result<(), ProtocolError> {
        let vote_phase_held = match self.status {
            IdentityContenderVotePollStatus::Joining => false,
            IdentityContenderVotePollStatus::Voting => true,
            IdentityContenderVotePollStatus::Resolved => {
                return Err(ProtocolError::CorruptedCodeExecution(
                    "trying to resolve an identity contender vote poll twice".to_string(),
                ))
            }
        };
        self.status = IdentityContenderVotePollStatus::Resolved;
        self.result = Some(IdentityContenderVotePollResultV0 {
            winner,
            vote_phase_held,
            finalization_block,
            resource_vote_choices,
        });
        Ok(())
    }
}
