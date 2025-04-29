use crate::errors::consensus::state::state_error::StateError;
use crate::errors::consensus::ConsensusError;
use crate::data_contract::TokenContractPosition;
use crate::identity::TimestampMillis;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Pre-programmed token distribution time {pre_programmed_timestamp}ms is in the past for token {token_position} in data contract {data_contract_id}. Current block time is {current_timestamp}.",
)]
#[platform_serialize(unversioned)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct PreProgrammedDistributionTimestampInPastError {
    pub data_contract_id: Identifier,
    pub token_position: TokenContractPosition,
    pub pre_programmed_timestamp: TimestampMillis,
    pub current_timestamp: TimestampMillis,
}

impl PreProgrammedDistributionTimestampInPastError {
    pub fn new(
        data_contract_id: Identifier,
        token_position: TokenContractPosition,
        pre_programmed_timestamp: TimestampMillis,
        current_timestamp: TimestampMillis,
    ) -> Self {
        Self {
            data_contract_id,
            token_position,
            pre_programmed_timestamp,
            current_timestamp,
        }
    }

    pub fn token_position(&self) -> TokenContractPosition {
        self.token_position
    }
}

impl From<PreProgrammedDistributionTimestampInPastError> for ConsensusError {
    fn from(err: PreProgrammedDistributionTimestampInPastError) -> Self {
        Self::StateError(StateError::PreProgrammedDistributionTimestampInPastError(
            err,
        ))
    }
}
