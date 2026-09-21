mod v0;

use crate::block::block_info::BlockInfo;
use crate::identity::TimestampMillis;
use crate::voting::contender_structs::FinalizedResourceVoteChoicesWithVoterInfo;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use derive_more::From;
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use platform_version::version::PlatformVersion;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
use std::fmt;

pub use v0::{IdentityContenderVotePollResultV0, IdentityContenderVotePollStoredInfoV0};

/// The phase an identity contender vote poll is in.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Default, Encode, Decode, DecodeUntrusted)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub enum IdentityContenderVotePollStatus {
    /// Contenders may join; nobody votes yet.
    #[default]
    Joining,
    /// The contenders are set; masternodes vote.
    Voting,
    /// The poll ended; its result is in the stored info.
    Resolved,
}

impl fmt::Display for IdentityContenderVotePollStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IdentityContenderVotePollStatus::Joining => write!(f, "Joining"),
            IdentityContenderVotePollStatus::Voting => write!(f, "Voting"),
            IdentityContenderVotePollStatus::Resolved => write!(f, "Resolved"),
        }
    }
}

/// The versioned state of an identity contender vote poll, kept under the poll in the votes
/// tree for as long as the poll runs and, with its result, after it ends.
#[derive(
    Debug,
    PartialEq,
    Eq,
    Clone,
    From,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    DecodeUntrusted,
)]
#[platform_serialize(unversioned)]
pub enum IdentityContenderVotePollStoredInfo {
    /// The first version.
    V0(IdentityContenderVotePollStoredInfoV0),
}

impl fmt::Display for IdentityContenderVotePollStoredInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IdentityContenderVotePollStoredInfo::V0(info) => write!(f, "V0({})", info),
        }
    }
}

impl IdentityContenderVotePollStoredInfo {
    /// The state of a poll that opens in `start_block`, whose join phase ends at
    /// `join_end_time_ms` and whose vote phase, if one runs, ends at `vote_end_time_ms`.
    pub fn new(
        start_block: BlockInfo,
        join_end_time_ms: TimestampMillis,
        vote_end_time_ms: TimestampMillis,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .voting_versions
            .identity_contender_vote_poll_stored_info_version
        {
            0 => Ok(IdentityContenderVotePollStoredInfoV0::new(
                start_block,
                join_end_time_ms,
                vote_end_time_ms,
            )
            .into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityContenderVotePollStoredInfo::new".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    /// Moves the poll from its join phase to its vote phase.
    pub fn start_vote_phase(&mut self) -> Result<(), ProtocolError> {
        match self {
            IdentityContenderVotePollStoredInfo::V0(v0) => v0.start_vote_phase(),
        }
    }

    /// Records how the poll ended and marks it resolved.
    pub fn resolve(
        &mut self,
        winner: Option<Identifier>,
        resource_vote_choices: Vec<FinalizedResourceVoteChoicesWithVoterInfo>,
        finalization_block: BlockInfo,
    ) -> Result<(), ProtocolError> {
        match self {
            IdentityContenderVotePollStoredInfo::V0(v0) => {
                v0.resolve(winner, resource_vote_choices, finalization_block)
            }
        }
    }
}

/// Read access to the stored info of an identity contender vote poll.
pub trait IdentityContenderVotePollStoredInfoV0Getters {
    /// The block the poll opened in.
    fn start_block(&self) -> &BlockInfo;
    /// When the join phase ends.
    fn join_end_time_ms(&self) -> TimestampMillis;
    /// When the vote phase ends, if one runs.
    fn vote_end_time_ms(&self) -> TimestampMillis;
    /// The phase the poll is in.
    fn status(&self) -> IdentityContenderVotePollStatus;
    /// The outcome, once the poll resolved.
    fn result(&self) -> Option<&IdentityContenderVotePollResultV0>;
    /// The winner, once the poll resolved with one.
    fn winner(&self) -> Option<Identifier>;
}

impl IdentityContenderVotePollStoredInfoV0Getters for IdentityContenderVotePollStoredInfo {
    fn start_block(&self) -> &BlockInfo {
        match self {
            IdentityContenderVotePollStoredInfo::V0(v0) => &v0.start_block,
        }
    }

    fn join_end_time_ms(&self) -> TimestampMillis {
        match self {
            IdentityContenderVotePollStoredInfo::V0(v0) => v0.join_end_time_ms,
        }
    }

    fn vote_end_time_ms(&self) -> TimestampMillis {
        match self {
            IdentityContenderVotePollStoredInfo::V0(v0) => v0.vote_end_time_ms,
        }
    }

    fn status(&self) -> IdentityContenderVotePollStatus {
        match self {
            IdentityContenderVotePollStoredInfo::V0(v0) => v0.status,
        }
    }

    fn result(&self) -> Option<&IdentityContenderVotePollResultV0> {
        match self {
            IdentityContenderVotePollStoredInfo::V0(v0) => v0.result.as_ref(),
        }
    }

    fn winner(&self) -> Option<Identifier> {
        self.result().and_then(|result| result.winner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serialization::{PlatformDeserializableUntrusted, PlatformSerializable};
    use crate::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;

    fn info() -> IdentityContenderVotePollStoredInfo {
        IdentityContenderVotePollStoredInfo::new(
            BlockInfo::default(),
            1_000,
            2_000,
            PlatformVersion::latest(),
        )
        .expect("expected the stored info")
    }

    #[test]
    fn should_open_in_the_join_phase() {
        let info = info();
        assert_eq!(info.status(), IdentityContenderVotePollStatus::Joining);
        assert_eq!(info.join_end_time_ms(), 1_000);
        assert_eq!(info.vote_end_time_ms(), 2_000);
        assert_eq!(info.result(), None);
        assert_eq!(info.winner(), None);
    }

    #[test]
    fn should_resolve_without_a_vote_phase_from_the_join_phase() {
        let mut info = info();
        let winner = Identifier::new([1; 32]);
        info.resolve(Some(winner), vec![], BlockInfo::default())
            .expect("expected to resolve");
        assert_eq!(info.status(), IdentityContenderVotePollStatus::Resolved);
        assert_eq!(info.winner(), Some(winner));
        assert!(!info.result().expect("result").vote_phase_held);
    }

    #[test]
    fn should_record_a_vote_phase_that_ran() {
        let mut info = info();
        info.start_vote_phase().expect("expected the vote phase");
        assert_eq!(info.status(), IdentityContenderVotePollStatus::Voting);
        let winner = Identifier::new([1; 32]);
        let choices = vec![
            FinalizedResourceVoteChoicesWithVoterInfo {
                resource_vote_choice: ResourceVoteChoice::TowardsIdentity(winner),
                voters: vec![(Identifier::new([7; 32]), 4), (Identifier::new([8; 32]), 1)],
            },
            FinalizedResourceVoteChoicesWithVoterInfo {
                resource_vote_choice: ResourceVoteChoice::Abstain,
                voters: vec![(Identifier::new([9; 32]), 1)],
            },
        ];
        info.resolve(Some(winner), choices, BlockInfo::default())
            .expect("expected to resolve");
        let result = info.result().expect("result");
        assert!(result.vote_phase_held);
        assert_eq!(
            result.tally_of(&ResourceVoteChoice::TowardsIdentity(winner)),
            5
        );
        assert_eq!(result.tally_of(&ResourceVoteChoice::Abstain), 1);
        assert_eq!(result.tally_of(&ResourceVoteChoice::Lock), 0);
    }

    #[test]
    fn should_refuse_to_start_the_vote_phase_twice_or_resolve_twice() {
        let mut info = info();
        info.start_vote_phase().expect("expected the vote phase");
        assert!(info.start_vote_phase().is_err());
        info.resolve(None, vec![], BlockInfo::default())
            .expect("expected to resolve");
        assert!(info.resolve(None, vec![], BlockInfo::default()).is_err());
    }

    #[test]
    fn should_round_trip_through_platform_serialization() {
        let mut info = info();
        info.start_vote_phase().expect("expected the vote phase");
        info.resolve(Some(Identifier::new([1; 32])), vec![], BlockInfo::default())
            .expect("expected to resolve");
        let bytes = info.serialize_to_bytes().expect("serialize");
        let recovered =
            IdentityContenderVotePollStoredInfo::deserialize_from_bytes_untrusted(&bytes)
                .expect("deserialize");
        assert_eq!(info, recovered);
    }
}
