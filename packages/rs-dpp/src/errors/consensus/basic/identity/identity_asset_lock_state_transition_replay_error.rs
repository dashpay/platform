use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use dashcore::Txid;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Bytes32;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Asset lock transaction {transaction_id} is trying to be replayed and will be discarded")]
#[platform_serialize(unversioned)]
pub struct IdentityAssetLockStateTransitionReplayError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    #[platform_serialize(with_serde)]
    #[bincode(with_serde)]
    transaction_id: Txid,

    output_index: usize,

    state_transition_id: Bytes32,
}

impl IdentityAssetLockStateTransitionReplayError {
    pub fn new(transaction_id: Txid, output_index: usize, state_transition_id: Bytes32) -> Self {
        Self {
            transaction_id,
            output_index,
            state_transition_id,
        }
    }

    pub fn transaction_id(&self) -> Txid {
        self.transaction_id
    }

    pub fn state_transition_id(&self) -> Bytes32 {
        self.state_transition_id
    }

    pub fn output_index(&self) -> usize {
        self.output_index
    }
}

impl From<IdentityAssetLockStateTransitionReplayError> for ConsensusError {
    fn from(err: IdentityAssetLockStateTransitionReplayError) -> Self {
        Self::BasicError(BasicError::IdentityAssetLockStateTransitionReplayError(err))
    }
}
