use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::group::GroupRequiredPower;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Debug, Error, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[platform_serialize(unversioned)]
#[error("The required group power {required_power} is invalid. It must be > 0 and <= {max_power}.")]
pub struct GroupRequiredPowerIsInvalidError {
    required_power: GroupRequiredPower,
    max_power: GroupRequiredPower,
}

impl GroupRequiredPowerIsInvalidError {
    pub fn new(required_power: GroupRequiredPower, max_power: GroupRequiredPower) -> Self {
        Self {
            required_power,
            max_power,
        }
    }

    pub fn required_power(&self) -> GroupRequiredPower {
        self.required_power
    }

    pub fn max_power(&self) -> GroupRequiredPower {
        self.max_power
    }
}

impl From<GroupRequiredPowerIsInvalidError> for ConsensusError {
    fn from(err: GroupRequiredPowerIsInvalidError) -> Self {
        Self::BasicError(BasicError::GroupRequiredPowerIsInvalidError(err))
    }
}
