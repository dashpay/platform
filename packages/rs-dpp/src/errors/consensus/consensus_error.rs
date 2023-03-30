use crate::consensus::basic::UnsupportedProtocolVersionError;
use platform_value::Value;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use thiserror::Error;

use crate::consensus::state::state_error::StateError;

use crate::consensus::fee::fee_error::FeeError;
use crate::consensus::signature::signature_error::SignatureError;

#[cfg(test)]
use crate::consensus::test_consensus_error::TestConsensusError;

use crate::errors::consensus::basic::BasicError;
use crate::ProtocolError;

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum ConsensusError {
    #[error(transparent)]
    StateError(StateError),

    #[error(transparent)]
    BasicError(BasicError),

    #[error(transparent)]
    #[serde(skip)] // TODO: Figure this out
    SignatureError(SignatureError),

    #[error(transparent)]
    #[serde(skip)] // TODO: Figure this out
    FeeError(FeeError),

    #[cfg(test)]
    #[cfg_attr(test, error(transparent))]
    #[serde(skip)] // TODO: Figure this out
    TestConsensusError(TestConsensusError),
}

#[cfg(test)]
impl From<TestConsensusError> for ConsensusError {
    fn from(error: TestConsensusError) -> Self {
        Self::TestConsensusError(error)
    }
}

impl TryFrom<&ConsensusError> for Value {
    type Error = ProtocolError;

    fn try_from(value: &ConsensusError) -> Result<Self, Self::Error> {
        platform_value::to_value(value).map_err(ProtocolError::ValueError)
    }
}

impl ConsensusError {
    pub fn serialize(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = Vec::new();

        ciborium::ser::into_writer(&self, &mut buffer).expect("should ...");

        buffer
    }

    pub fn unserialize(cbor: Vec<u8>) -> Self {
        ciborium::de::from_reader(cbor.as_slice()).expect("should ...")
    }
}
