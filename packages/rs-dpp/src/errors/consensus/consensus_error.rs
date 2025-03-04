use crate::errors::ProtocolError;
use bincode;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};

use crate::errors::consensus::state::state_error::StateError;

use crate::errors::consensus::fee::fee_error::FeeError;
use crate::errors::consensus::signature::SignatureError;

#[cfg(test)]
use crate::errors::consensus::test_consensus_error::TestConsensusError;

use crate::errors::consensus::basic::BasicError;

// TODO It must be versioned as all other serializable types

#[derive(
    thiserror::Error,
    Debug,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserialize,
    Clone,
    PartialEq,
)]
#[platform_serialize(limit = 2000)]
#[error(transparent)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
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

#[cfg(test)]
impl From<TestConsensusError> for ConsensusError {
    fn from(error: TestConsensusError) -> Self {
        Self::TestConsensusError(error)
    }
}
