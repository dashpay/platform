use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::voting::vote_polls::VotePoll;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Masternode vote is already present for masternode {pro_tx_hash} voting for {vote_poll}")]
#[platform_serialize(unversioned)]
pub struct MasternodeVoteAlreadyPresentError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    pro_tx_hash: Identifier,
    vote_poll: VotePoll,
}

impl MasternodeVoteAlreadyPresentError {
    pub fn new(pro_tx_hash: Identifier, vote_poll: VotePoll) -> Self {
        Self {
            pro_tx_hash,
            vote_poll,
        }
    }

    pub fn pro_tx_hash(&self) -> Identifier {
        self.pro_tx_hash
    }

    pub fn vote_poll(&self) -> &VotePoll {
        &self.vote_poll
    }
}

impl From<MasternodeVoteAlreadyPresentError> for ConsensusError {
    fn from(err: MasternodeVoteAlreadyPresentError) -> Self {
        Self::StateError(StateError::MasternodeVoteAlreadyPresentError(err))
    }
}
