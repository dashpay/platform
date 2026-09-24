pub mod v0;

use crate::block::block_info::BlockInfo;
use crate::voting::vote_info_storage::yes_no_vote_poll_stored_info::v0::YesNoVotePollStoredInfoV0;
use crate::voting::vote_polls::yes_no_vote_poll::VotingPower;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use derive_more::From;
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_version::version::PlatformVersion;
use std::fmt;

/// How a yes/no vote poll ended: the final tallies and whether they passed the poll's rule.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Default, Encode, Decode, DecodeUntrusted)]
pub struct YesNoVotePollResult {
    /// Whether the poll passed.
    pub passed: bool,
    /// The yes voting power at the end.
    pub yes_voting_power: VotingPower,
    /// The no voting power at the end.
    pub no_voting_power: VotingPower,
    /// The abstaining voting power at the end. Recorded, but it plays no part in the result.
    pub abstain_voting_power: VotingPower,
    /// The yes plus no voting power the poll needed, resolved in the block that closed it (a
    /// share of the total is measured then).
    pub required_voting_power: VotingPower,
    /// The block the poll opened in.
    pub start_block: BlockInfo,
    /// The block the poll was closed in.
    pub finalization_block: BlockInfo,
}

impl fmt::Display for YesNoVotePollResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "YesNoVotePollResult {{ passed: {}, yes: {}, no: {}, abstain: {}, required: {}, start_block: {}, finalization_block: {} }}",
            self.passed,
            self.yes_voting_power,
            self.no_voting_power,
            self.abstain_voting_power,
            self.required_voting_power,
            self.start_block,
            self.finalization_block
        )
    }
}

/// Where a yes/no vote poll is in its life. A poll runs once: finished, it stays finished, and
/// its result is the record of the decision.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Encode, Decode, DecodeUntrusted)]
pub enum YesNoVotePollStatus {
    /// Votes are being taken since this block.
    Started(BlockInfo),
    /// The poll was closed with this result.
    Finished(YesNoVotePollResult),
}

impl fmt::Display for YesNoVotePollStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            YesNoVotePollStatus::Started(block_info) => write!(f, "Started({})", block_info),
            YesNoVotePollStatus::Finished(result) => write!(f, "Finished({})", result),
        }
    }
}

/// What Drive stores about a yes/no vote poll beside its votes.
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
pub enum YesNoVotePollStoredInfo {
    V0(YesNoVotePollStoredInfoV0),
}

impl fmt::Display for YesNoVotePollStoredInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            YesNoVotePollStoredInfo::V0(info) => write!(f, "V0({})", info),
        }
    }
}

impl YesNoVotePollStoredInfo {
    /// The stored info of a poll that opens in `start_block`.
    pub fn new(
        start_block: BlockInfo,
        platform_version: &PlatformVersion,
    ) -> Result<YesNoVotePollStoredInfo, ProtocolError> {
        match platform_version
            .dpp
            .voting_versions
            .yes_no_vote_poll_stored_info_version
        {
            0 => Ok(YesNoVotePollStoredInfoV0::new(start_block).into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "YesNoVotePollStoredInfo::new".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    /// Rewrites the stored info in the version the platform version wants stored.
    pub fn update_to_latest_version(
        self,
        platform_version: &PlatformVersion,
    ) -> Result<YesNoVotePollStoredInfo, ProtocolError> {
        match platform_version
            .dpp
            .voting_versions
            .yes_no_vote_poll_stored_info_version
        {
            0 => match self {
                YesNoVotePollStoredInfo::V0(_) => Ok(self),
            },
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "YesNoVotePollStoredInfo::update_to_latest_version".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    /// Closes a started poll with its final tallies. `passed` is the poll's own verdict on
    /// them; the stored info records it without judging.
    pub fn finalize_vote_poll(
        &mut self,
        yes_voting_power: VotingPower,
        no_voting_power: VotingPower,
        abstain_voting_power: VotingPower,
        required_voting_power: VotingPower,
        passed: bool,
        finalization_block: BlockInfo,
    ) -> Result<YesNoVotePollResult, ProtocolError> {
        match self {
            YesNoVotePollStoredInfo::V0(v0) => v0.finalize_vote_poll(
                yes_voting_power,
                no_voting_power,
                abstain_voting_power,
                required_voting_power,
                passed,
                finalization_block,
            ),
        }
    }

    /// Where the poll is in its life.
    pub fn status(&self) -> &YesNoVotePollStatus {
        match self {
            YesNoVotePollStoredInfo::V0(v0) => &v0.status,
        }
    }

    /// The result, once the poll has finished.
    pub fn result(&self) -> Option<&YesNoVotePollResult> {
        match self.status() {
            YesNoVotePollStatus::Started(_) => None,
            YesNoVotePollStatus::Finished(result) => Some(result),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serialization::{PlatformDeserializableTrusted, PlatformSerializable};

    #[test]
    fn should_finalize_a_started_poll_once() {
        let platform_version = PlatformVersion::latest();
        let start_block = BlockInfo::default_with_time(10);
        let mut stored_info =
            YesNoVotePollStoredInfo::new(start_block, platform_version).expect("stored info");
        assert_eq!(
            stored_info.status(),
            &YesNoVotePollStatus::Started(start_block)
        );
        assert!(stored_info.result().is_none());

        let finalization_block = BlockInfo::default_with_time(20);
        let result = stored_info
            .finalize_vote_poll(300, 100, 7, 400, true, finalization_block)
            .expect("finalize");
        assert_eq!(
            result,
            YesNoVotePollResult {
                passed: true,
                yes_voting_power: 300,
                no_voting_power: 100,
                abstain_voting_power: 7,
                required_voting_power: 400,
                start_block,
                finalization_block,
            }
        );
        assert_eq!(stored_info.result(), Some(&result));

        // A finished poll is not finalized twice.
        assert!(stored_info
            .finalize_vote_poll(1, 1, 1, 400, false, finalization_block)
            .is_err());
    }

    #[test]
    fn should_round_trip_through_platform_serialization() {
        let platform_version = PlatformVersion::latest();
        let mut stored_info =
            YesNoVotePollStoredInfo::new(BlockInfo::default_with_time(10), platform_version)
                .expect("stored info");
        stored_info
            .finalize_vote_poll(5, 4, 3, 400, false, BlockInfo::default_with_time(20))
            .expect("finalize");
        let bytes = stored_info.serialize_to_bytes().expect("serialize");
        let recovered =
            YesNoVotePollStoredInfo::deserialize_from_bytes_trusted(&bytes).expect("deserialize");
        assert_eq!(recovered, stored_info);
    }
}
