use bincode;
use bincode::{config, Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::consensus::state::state_error::StateError;

use crate::consensus::fee::fee_error::FeeError;
use crate::consensus::signature::SignatureError;
#[cfg(feature = "validation")]
use crate::consensus::state::data_trigger::DataTriggerError;

#[cfg(test)]
use crate::consensus::test_consensus_error::TestConsensusError;

use crate::errors::consensus::basic::BasicError;
use crate::ProtocolError;

// TODO It must be versioned as all other serializable types

#[derive(
    Error, Debug, Serialize, Deserialize, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[platform_serialize(limit = 2000)]
pub enum ConsensusError {
    /*

    DO NOT CHANGE ORDER OF VARIANTS WITHOUT INTRODUCING OF NEW VERSION

    */
    #[error("default error")]
    DefaultError,

    #[error(transparent)]
    BasicError(BasicError),

    #[error(transparent)]
    StateError(StateError),

    #[error(transparent)]
    SignatureError(SignatureError),

    #[error(transparent)]
    FeeError(FeeError),

    #[cfg(test)]
    #[cfg_attr(test, error(transparent))]
    TestConsensusError(TestConsensusError),
}

#[cfg(feature = "validation")]
impl From<DataTriggerError> for ConsensusError {
    fn from(error: DataTriggerError) -> Self {
        Self::StateError(StateError::DataTriggerError(error))
    }
}

#[cfg(test)]
impl From<TestConsensusError> for ConsensusError {
    fn from(error: TestConsensusError) -> Self {
        Self::TestConsensusError(error)
    }
}
