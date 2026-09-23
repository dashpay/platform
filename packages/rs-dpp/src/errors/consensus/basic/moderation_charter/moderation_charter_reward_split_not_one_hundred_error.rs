use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

#[derive(
    Error,
    Debug,
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    DecodeUntrusted,
)]
#[error(
    "The moderation charter's reward split of {leader}% to the leader, {equal}% equally and {actions}% by action count sums to {}%, it must sum to 100%",
    *leader as u16 + *equal as u16 + *actions as u16
)]
#[platform_serialize(unversioned)]
pub struct ModerationCharterRewardSplitNotOneHundredError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    leader: u8,
    equal: u8,
    actions: u8,
}

impl ModerationCharterRewardSplitNotOneHundredError {
    pub fn new(leader: u8, equal: u8, actions: u8) -> Self {
        Self {
            leader,
            equal,
            actions,
        }
    }

    pub fn leader(&self) -> u8 {
        self.leader
    }

    pub fn equal(&self) -> u8 {
        self.equal
    }

    pub fn actions(&self) -> u8 {
        self.actions
    }
}

impl From<ModerationCharterRewardSplitNotOneHundredError> for ConsensusError {
    fn from(err: ModerationCharterRewardSplitNotOneHundredError) -> Self {
        Self::BasicError(BasicError::ModerationCharterRewardSplitNotOneHundredError(
            err,
        ))
    }
}
