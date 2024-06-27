use crate::block::block_info::BlockInfo;
use crate::voting::contender_structs::{
    ContenderWithSerializedDocument, ContenderWithSerializedDocumentV0,
    FinalizedResourceVoteChoicesWithVoterInfo,
};
use crate::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use crate::voting::vote_info_storage::contested_document_vote_poll_stored_info::ContestedDocumentVotePollStatus;
use crate::voting::vote_info_storage::contested_document_vote_poll_winner_info::ContestedDocumentVotePollWinnerInfo;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_value::Identifier;
use std::fmt;

// We can have multiple rounds of voting, after an unlock for example
#[derive(Debug, PartialEq, Eq, Clone, Default, Encode, Decode)]
pub struct ContestedDocumentVotePollStoredInfoVoteEventV0 {
    /// The list of contenders returned by the query.
    pub resource_vote_choices: Vec<FinalizedResourceVoteChoicesWithVoterInfo>,
    /// Start Block
    pub start_block: BlockInfo,
    /// Finalization Block
    pub finalization_block: BlockInfo,
    /// Winner info
    pub winner: ContestedDocumentVotePollWinnerInfo,
}

impl fmt::Display for ContestedDocumentVotePollStoredInfoVoteEventV0 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let resource_vote_choices_str: Vec<String> = self
            .resource_vote_choices
            .iter()
            .map(|v| v.to_string())
            .collect();
        write!(
            f,
            "ContestedDocumentVotePollStoredInfoVoteEventV0 {{ resource_vote_choices: [{}], start_block: {}, finalization_block: {}, winner: {} }}",
            resource_vote_choices_str.join(", "),
            self.start_block,
            self.finalization_block,
            self.winner
        )
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Default, Encode, Decode)]
pub struct ContestedDocumentVotePollStoredInfoV0 {
    /// The list of contenders returned by the query.
    pub finalized_events: Vec<ContestedDocumentVotePollStoredInfoVoteEventV0>,
    /// Start Block
    pub vote_poll_status: ContestedDocumentVotePollStatus,
    /// Locked count, aka how many times has this previously been locked
    pub locked_count: u16,
}

impl fmt::Display for ContestedDocumentVotePollStoredInfoV0 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let finalized_events_str: Vec<String> = self
            .finalized_events
            .iter()
            .map(|v| v.to_string())
            .collect();
        write!(
            f,
            "ContestedDocumentVotePollStoredInfoV0 {{ finalized_events: [{}], vote_poll_status: {}, locked_count: {} }}",
            finalized_events_str.join(", "),
            self.vote_poll_status,
            self.locked_count
        )
    }
}

impl ContestedDocumentVotePollStoredInfoV0 {
    pub fn new(start_block: BlockInfo) -> ContestedDocumentVotePollStoredInfoV0 {
        ContestedDocumentVotePollStoredInfoV0 {
            finalized_events: vec![],
            vote_poll_status: ContestedDocumentVotePollStatus::Started(start_block),
            locked_count: 0,
        }
    }
}

impl ContestedDocumentVotePollStoredInfoVoteEventV0 {
    pub fn new(
        resource_vote_choices: Vec<FinalizedResourceVoteChoicesWithVoterInfo>,
        start_block: BlockInfo,
        finalization_block: BlockInfo,
        winner: ContestedDocumentVotePollWinnerInfo,
    ) -> ContestedDocumentVotePollStoredInfoVoteEventV0 {
        ContestedDocumentVotePollStoredInfoVoteEventV0 {
            resource_vote_choices,
            start_block,
            finalization_block,
            winner,
        }
    }
}

impl ContestedDocumentVotePollStoredInfoV0 {
    /// This will finalize the current vote poll.
    /// However, if this results in it being locked, then it is possible to unlock in the future.
    pub fn finalize_vote_poll(
        &mut self,
        resource_vote_choices: Vec<FinalizedResourceVoteChoicesWithVoterInfo>,
        finalization_block: BlockInfo,
        winner: ContestedDocumentVotePollWinnerInfo,
    ) -> Result<(), ProtocolError> {
        let ContestedDocumentVotePollStatus::Started(started_block) = self.vote_poll_status else {
            return Err(ProtocolError::CorruptedCodeExecution(
                "trying to finalized vote poll that hasn't started".to_string(),
            ));
        };
        self.finalized_events
            .push(ContestedDocumentVotePollStoredInfoVoteEventV0::new(
                resource_vote_choices,
                started_block,
                finalization_block,
                winner,
            ));
        match winner {
            ContestedDocumentVotePollWinnerInfo::NoWinner => {
                if self.locked_count > 0 {
                    // We return it to being in the locked position
                    self.vote_poll_status = ContestedDocumentVotePollStatus::Locked;
                } else {
                    self.vote_poll_status = ContestedDocumentVotePollStatus::NotStarted;
                }
            }
            ContestedDocumentVotePollWinnerInfo::WonByIdentity(identifier) => {
                self.vote_poll_status = ContestedDocumentVotePollStatus::Awarded(identifier);
            }
            ContestedDocumentVotePollWinnerInfo::Locked => {
                self.locked_count += 1;
                self.vote_poll_status = ContestedDocumentVotePollStatus::Locked;
            }
        }
        Ok(())
    }
}

pub trait ContestedDocumentVotePollStoredInfoV0Getters {
    fn last_resource_vote_choices(&self)
        -> Option<&Vec<FinalizedResourceVoteChoicesWithVoterInfo>>;
    fn awarded_block(&self) -> Option<BlockInfo>;
    fn current_start_block(&self) -> Option<BlockInfo>;
    fn last_finalization_block(&self) -> Option<BlockInfo>;
    fn winner(&self) -> ContestedDocumentVotePollWinnerInfo;

    fn last_locked_votes(&self) -> Option<u32>;

    fn last_locked_voters(&self) -> Option<Vec<(Identifier, u8)>>;

    fn last_abstain_votes(&self) -> Option<u32>;

    fn last_abstain_voters(&self) -> Option<Vec<(Identifier, u8)>>;
    fn contender_votes_in_vec_of_contender_with_serialized_document(
        &self,
    ) -> Option<Vec<ContenderWithSerializedDocument>>;
    fn vote_poll_status(&self) -> ContestedDocumentVotePollStatus;
    fn vote_poll_status_ref(&self) -> &ContestedDocumentVotePollStatus;
}

impl ContestedDocumentVotePollStoredInfoV0Getters for ContestedDocumentVotePollStoredInfoV0 {
    fn last_resource_vote_choices(
        &self,
    ) -> Option<&Vec<FinalizedResourceVoteChoicesWithVoterInfo>> {
        self.finalized_events
            .last()
            .map(|event| &event.resource_vote_choices)
    }

    fn awarded_block(&self) -> Option<BlockInfo> {
        if matches!(
            self.vote_poll_status,
            ContestedDocumentVotePollStatus::Awarded(_)
        ) {
            self.finalized_events
                .last()
                .map(|event| event.finalization_block)
        } else {
            None
        }
    }

    fn current_start_block(&self) -> Option<BlockInfo> {
        if let ContestedDocumentVotePollStatus::Started(start_block) = self.vote_poll_status {
            Some(start_block)
        } else {
            None
        }
    }

    fn last_finalization_block(&self) -> Option<BlockInfo> {
        self.finalized_events
            .last()
            .map(|event| event.finalization_block)
    }

    fn winner(&self) -> ContestedDocumentVotePollWinnerInfo {
        match self.vote_poll_status {
            ContestedDocumentVotePollStatus::NotStarted => {
                ContestedDocumentVotePollWinnerInfo::NoWinner
            }
            ContestedDocumentVotePollStatus::Awarded(identifier) => {
                ContestedDocumentVotePollWinnerInfo::WonByIdentity(identifier)
            }
            ContestedDocumentVotePollStatus::Locked => ContestedDocumentVotePollWinnerInfo::Locked,
            ContestedDocumentVotePollStatus::Started(_) => {
                ContestedDocumentVotePollWinnerInfo::NoWinner
            }
        }
    }

    fn last_locked_votes(&self) -> Option<u32> {
        self.last_resource_vote_choices()
            .map(|resource_vote_choices| {
                resource_vote_choices
                    .iter()
                    .filter(|choice| {
                        matches!(choice.resource_vote_choice, ResourceVoteChoice::Lock)
                    })
                    .map(|choice| {
                        let sum: u32 = choice
                            .voters
                            .iter()
                            .map(|(_, strength)| *strength as u32)
                            .sum();
                        sum
                    })
                    .sum()
            })
    }

    fn last_locked_voters(&self) -> Option<Vec<(Identifier, u8)>> {
        self.last_resource_vote_choices()
            .map(|resource_vote_choices| {
                resource_vote_choices
                    .iter()
                    .filter(|choice| {
                        matches!(choice.resource_vote_choice, ResourceVoteChoice::Lock)
                    })
                    .flat_map(|choice| choice.voters.clone())
                    .collect()
            })
    }

    fn last_abstain_votes(&self) -> Option<u32> {
        self.last_resource_vote_choices()
            .map(|resource_vote_choices| {
                resource_vote_choices
                    .iter()
                    .filter(|choice| {
                        matches!(choice.resource_vote_choice, ResourceVoteChoice::Abstain)
                    })
                    .map(|choice| {
                        let sum: u32 = choice
                            .voters
                            .iter()
                            .map(|(_, strength)| *strength as u32)
                            .sum();
                        sum
                    })
                    .sum()
            })
    }

    fn last_abstain_voters(&self) -> Option<Vec<(Identifier, u8)>> {
        self.last_resource_vote_choices()
            .map(|resource_vote_choices| {
                resource_vote_choices
                    .iter()
                    .filter(|choice| {
                        matches!(choice.resource_vote_choice, ResourceVoteChoice::Abstain)
                    })
                    .flat_map(|choice| choice.voters.clone())
                    .collect()
            })
    }

    fn contender_votes_in_vec_of_contender_with_serialized_document(
        &self,
    ) -> Option<Vec<ContenderWithSerializedDocument>> {
        self.last_resource_vote_choices()
            .map(|resource_vote_choices| {
                resource_vote_choices
                    .iter()
                    .filter_map(|choice| {
                        if let ResourceVoteChoice::TowardsIdentity(identity_id) =
                            &choice.resource_vote_choice
                        {
                            let vote_tally: u32 = choice
                                .voters
                                .iter()
                                .map(|(_, strength)| *strength as u32)
                                .sum();
                            Some(
                                ContenderWithSerializedDocumentV0 {
                                    identity_id: *identity_id,
                                    serialized_document: None,
                                    vote_tally: Some(vote_tally),
                                }
                                .into(),
                            )
                        } else {
                            None
                        }
                    })
                    .collect()
            })
    }

    fn vote_poll_status(&self) -> ContestedDocumentVotePollStatus {
        self.vote_poll_status
    }

    fn vote_poll_status_ref(&self) -> &ContestedDocumentVotePollStatus {
        &self.vote_poll_status
    }
}
