use bincode;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::consensus::state::state_error::StateError;

use crate::consensus::fee::fee_error::FeeError;
use crate::consensus::signature::SignatureError;

#[cfg(test)]
use crate::consensus::test_consensus_error::TestConsensusError;

use crate::errors::consensus::basic::BasicError;
use crate::ProtocolError;

// TODO It must be versioned as all other serializable types

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum ConsensusError {
    /*

    DO NOT CHANGE ORDER OF VARIANTS WITHOUT INTRODUCING OF NEW VERSION

    */
    #[error("default error")]
    DefaultError,

    #[error(transparent)]
    BasicError(BasicError),

    #[error("system error: {0}")]
    SystemError(String),

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

impl ConsensusError {
    pub fn serialize(&self) -> Result<Vec<u8>, ProtocolError> {
        // TODO(v0.24-backport): use new bincode API?
        let config = bincode::config::legacy()
            .with_variable_int_encoding()
            .with_big_endian();

        bincode::serde::encode_to_vec(self, config).map_err(|e| {
            ProtocolError::EncodingError(format!("unable to serialize consensus error: {e}"))
        })
    }

    pub fn deserialize(bytes: &[u8]) -> Result<ConsensusError, ProtocolError> {
        // TODO(v0.24-backport): use new bincode API?
        let config = bincode::config::legacy()
            .with_variable_int_encoding()
            .with_big_endian();

        bincode::serde::decode_borrowed_from_slice(bytes, config).map_err(|e| {
            ProtocolError::EncodingError(format!("unable to deserialize consensus error: {e}"))
        })
    }
}
