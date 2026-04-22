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

#[cfg(test)]
mod tests {
    use super::*;

    fn start_block(time: u64, height: u64) -> BlockInfo {
        BlockInfo {
            time_ms: time,
            height,
            ..BlockInfo::default()
        }
    }

    #[test]
    fn new_sets_status_to_started() {
        let sb = start_block(1, 1);
        let info = ContestedDocumentVotePollStoredInfoV0::new(sb);
        assert!(info.finalized_events.is_empty());
        assert_eq!(info.locked_count, 0);
        match info.vote_poll_status {
            ContestedDocumentVotePollStatus::Started(b) => assert_eq!(b, sb),
            other => panic!("expected Started, got {:?}", other),
        }
    }

    #[test]
    fn finalize_errors_when_not_started() {
        let mut info = ContestedDocumentVotePollStoredInfoV0 {
            finalized_events: vec![],
            vote_poll_status: ContestedDocumentVotePollStatus::NotStarted,
            locked_count: 0,
        };
        let err = info
            .finalize_vote_poll(
                vec![],
                start_block(2, 2),
                ContestedDocumentVotePollWinnerInfo::NoWinner,
            )
            .expect_err("should fail when not started");
        match err {
            ProtocolError::CorruptedCodeExecution(msg) => {
                assert!(msg.contains("hasn't started"));
            }
            _ => panic!("expected CorruptedCodeExecution"),
        }
    }

    #[test]
    fn finalize_errors_when_locked() {
        let mut info = ContestedDocumentVotePollStoredInfoV0 {
            finalized_events: vec![],
            vote_poll_status: ContestedDocumentVotePollStatus::Locked,
            locked_count: 1,
        };
        let err = info
            .finalize_vote_poll(
                vec![],
                start_block(2, 2),
                ContestedDocumentVotePollWinnerInfo::NoWinner,
            )
            .expect_err("should fail when locked");
        assert!(matches!(err, ProtocolError::CorruptedCodeExecution(_)));
    }

    #[test]
    fn finalize_no_winner_from_started_without_prior_locks_returns_not_started() {
        let sb = start_block(1, 1);
        let mut info = ContestedDocumentVotePollStoredInfoV0::new(sb);
        info.finalize_vote_poll(
            vec![],
            start_block(2, 2),
            ContestedDocumentVotePollWinnerInfo::NoWinner,
        )
        .expect("should finalize");
        assert_eq!(info.finalized_events.len(), 1);
        assert!(matches!(
            info.vote_poll_status,
            ContestedDocumentVotePollStatus::NotStarted
        ));
    }

    #[test]
    fn finalize_no_winner_with_prior_locks_returns_locked() {
        let sb = start_block(1, 1);
        let mut info = ContestedDocumentVotePollStoredInfoV0::new(sb);
        info.locked_count = 2;
        info.finalize_vote_poll(
            vec![],
            start_block(2, 2),
            ContestedDocumentVotePollWinnerInfo::NoWinner,
        )
        .expect("should finalize");
        assert!(matches!(
            info.vote_poll_status,
            ContestedDocumentVotePollStatus::Locked
        ));
        // locked_count is preserved (not incremented for NoWinner)
        assert_eq!(info.locked_count, 2);
    }

    #[test]
    fn finalize_with_identity_winner_returns_awarded() {
        let sb = start_block(1, 1);
        let mut info = ContestedDocumentVotePollStoredInfoV0::new(sb);
        let winner_id = Identifier::new([7u8; 32]);
        info.finalize_vote_poll(
            vec![],
            start_block(2, 2),
            ContestedDocumentVotePollWinnerInfo::WonByIdentity(winner_id),
        )
        .expect("should finalize");
        match info.vote_poll_status {
            ContestedDocumentVotePollStatus::Awarded(id) => assert_eq!(id, winner_id),
            other => panic!("expected Awarded, got {:?}", other),
        }
        // awarded_block should now return the finalization block
        assert_eq!(info.awarded_block(), Some(start_block(2, 2)));
    }

    #[test]
    fn finalize_with_locked_winner_increments_lock_count() {
        let sb = start_block(1, 1);
        let mut info = ContestedDocumentVotePollStoredInfoV0::new(sb);
        assert_eq!(info.locked_count, 0);
        info.finalize_vote_poll(
            vec![],
            start_block(2, 2),
            ContestedDocumentVotePollWinnerInfo::Locked,
        )
        .expect("should finalize");
        assert_eq!(info.locked_count, 1);
        assert!(matches!(
            info.vote_poll_status,
            ContestedDocumentVotePollStatus::Locked
        ));
    }

    #[test]
    fn status_awarded_or_locked_helper() {
        assert!(ContestedDocumentVotePollStatus::Locked.awarded_or_locked());
        assert!(
            ContestedDocumentVotePollStatus::Awarded(Identifier::default()).awarded_or_locked()
        );
        assert!(!ContestedDocumentVotePollStatus::NotStarted.awarded_or_locked());
        assert!(
            !ContestedDocumentVotePollStatus::Started(BlockInfo::default()).awarded_or_locked()
        );
    }

    #[test]
    fn current_start_block_only_when_started() {
        let sb = start_block(1, 1);
        let info = ContestedDocumentVotePollStoredInfoV0::new(sb);
        assert_eq!(info.current_start_block(), Some(sb));

        let awarded = ContestedDocumentVotePollStoredInfoV0 {
            finalized_events: vec![],
            vote_poll_status: ContestedDocumentVotePollStatus::Awarded(Identifier::default()),
            locked_count: 0,
        };
        assert_eq!(awarded.current_start_block(), None);

        let locked = ContestedDocumentVotePollStoredInfoV0 {
            finalized_events: vec![],
            vote_poll_status: ContestedDocumentVotePollStatus::Locked,
            locked_count: 1,
        };
        assert_eq!(locked.current_start_block(), None);
    }

    #[test]
    fn awarded_block_none_when_not_awarded() {
        let info = ContestedDocumentVotePollStoredInfoV0::new(start_block(1, 1));
        assert_eq!(info.awarded_block(), None);
    }

    #[test]
    fn winner_getter_maps_statuses() {
        // NotStarted -> NoWinner
        let not_started = ContestedDocumentVotePollStoredInfoV0 {
            finalized_events: vec![],
            vote_poll_status: ContestedDocumentVotePollStatus::NotStarted,
            locked_count: 0,
        };
        assert_eq!(
            not_started.winner(),
            ContestedDocumentVotePollWinnerInfo::NoWinner
        );
        // Locked -> Locked
        let locked = ContestedDocumentVotePollStoredInfoV0 {
            finalized_events: vec![],
            vote_poll_status: ContestedDocumentVotePollStatus::Locked,
            locked_count: 1,
        };
        assert_eq!(locked.winner(), ContestedDocumentVotePollWinnerInfo::Locked);
        // Started -> NoWinner
        let started = ContestedDocumentVotePollStoredInfoV0::new(start_block(1, 1));
        assert_eq!(
            started.winner(),
            ContestedDocumentVotePollWinnerInfo::NoWinner
        );
        // Awarded -> WonByIdentity
        let wid = Identifier::new([8u8; 32]);
        let awarded = ContestedDocumentVotePollStoredInfoV0 {
            finalized_events: vec![],
            vote_poll_status: ContestedDocumentVotePollStatus::Awarded(wid),
            locked_count: 0,
        };
        assert_eq!(
            awarded.winner(),
            ContestedDocumentVotePollWinnerInfo::WonByIdentity(wid)
        );
    }

    #[test]
    fn last_locked_and_abstain_aggregations() {
        let sb = start_block(1, 1);
        let mut info = ContestedDocumentVotePollStoredInfoV0::new(sb);
        let v1 = Identifier::new([1u8; 32]);
        let v2 = Identifier::new([2u8; 32]);
        let v3 = Identifier::new([3u8; 32]);
        // Two lock voters (strength 1 and 4), one abstain voter (strength 2)
        let choices = vec![
            FinalizedResourceVoteChoicesWithVoterInfo {
                resource_vote_choice: ResourceVoteChoice::Lock,
                voters: vec![(v1, 1), (v2, 4)],
            },
            FinalizedResourceVoteChoicesWithVoterInfo {
                resource_vote_choice: ResourceVoteChoice::Abstain,
                voters: vec![(v3, 2)],
            },
            FinalizedResourceVoteChoicesWithVoterInfo {
                resource_vote_choice: ResourceVoteChoice::TowardsIdentity(Identifier::new(
                    [9u8; 32],
                )),
                voters: vec![(v1, 3)],
            },
        ];
        info.finalize_vote_poll(
            choices,
            start_block(2, 2),
            ContestedDocumentVotePollWinnerInfo::NoWinner,
        )
        .expect("should finalize");

        assert_eq!(info.last_locked_votes(), Some(5));
        assert_eq!(info.last_abstain_votes(), Some(2));
        let locked_voters = info.last_locked_voters().expect("some");
        assert!(locked_voters.contains(&(v1, 1)));
        assert!(locked_voters.contains(&(v2, 4)));
        let abstain_voters = info.last_abstain_voters().expect("some");
        assert_eq!(abstain_voters, vec![(v3, 2)]);

        let towards_only = info
            .contender_votes_in_vec_of_contender_with_serialized_document()
            .expect("some");
        assert_eq!(towards_only.len(), 1);
        let first = &towards_only[0];
        assert_eq!(first.identity_id(), Identifier::new([9u8; 32]));
        assert_eq!(first.vote_tally(), Some(3));
    }

    #[test]
    fn last_accessors_none_when_no_finalized_events() {
        let info = ContestedDocumentVotePollStoredInfoV0::new(start_block(1, 1));
        assert!(info.last_resource_vote_choices().is_none());
        assert!(info.last_locked_votes().is_none());
        assert!(info.last_abstain_votes().is_none());
        assert!(info.last_finalization_block().is_none());
        assert!(info
            .contender_votes_in_vec_of_contender_with_serialized_document()
            .is_none());
    }

    #[test]
    fn display_contains_key_fields() {
        let info = ContestedDocumentVotePollStoredInfoV0::new(start_block(10, 5));
        let rendered = info.to_string();
        assert!(rendered.contains("finalized_events"));
        assert!(rendered.contains("vote_poll_status"));
        assert!(rendered.contains("locked_count: 0"));

        let status_str = ContestedDocumentVotePollStatus::NotStarted.to_string();
        assert_eq!(status_str, "NotStarted");

        let locked_str = ContestedDocumentVotePollStatus::Locked.to_string();
        assert_eq!(locked_str, "Locked");

        let awarded_str =
            ContestedDocumentVotePollStatus::Awarded(Identifier::new([1u8; 32])).to_string();
        assert!(awarded_str.starts_with("Awarded("));
        let started_str =
            ContestedDocumentVotePollStatus::Started(BlockInfo::default()).to_string();
        assert!(started_str.starts_with("Started("));
    }

    #[test]
    fn status_encode_decode_roundtrip() {
        use bincode::config;
        let cfg = config::standard();
        for status in [
            ContestedDocumentVotePollStatus::NotStarted,
            ContestedDocumentVotePollStatus::Locked,
            ContestedDocumentVotePollStatus::Awarded(Identifier::new([5u8; 32])),
            ContestedDocumentVotePollStatus::Started(BlockInfo::default()),
        ] {
            let bytes = bincode::encode_to_vec(status, cfg).expect("encode");
            let (decoded, _): (ContestedDocumentVotePollStatus, _) =
                bincode::decode_from_slice(&bytes, cfg).expect("decode");
            assert_eq!(decoded, status);
        }
    }

    #[test]
    fn vote_event_new_preserves_fields() {
        let sb = start_block(1, 1);
        let fb = start_block(2, 2);
        let winner = ContestedDocumentVotePollWinnerInfo::Locked;
        let event = ContestedDocumentVotePollStoredInfoVoteEventV0::new(vec![], sb, fb, winner);
        assert!(event.resource_vote_choices.is_empty());
        assert_eq!(event.start_block, sb);
        assert_eq!(event.finalization_block, fb);
        assert_eq!(event.winner, winner);
    }

    #[test]
    fn vote_event_display_contains_winner() {
        let sb = start_block(1, 1);
        let fb = start_block(2, 2);
        let event = ContestedDocumentVotePollStoredInfoVoteEventV0::new(
            vec![],
            sb,
            fb,
            ContestedDocumentVotePollWinnerInfo::Locked,
        );
        let rendered = event.to_string();
        assert!(rendered.contains("winner: Locked"));
    }
}
