use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::fee::Credits;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("asset-lock surplus {surplus} exceeds the implicit fee cap {cap}; set a surplus_output address to receive the remainder")]
#[platform_serialize(unversioned)]
pub struct ShieldedImplicitFeeCapExceededError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    /// The asset-lock surplus (in credits) that would be implicitly donated to the fee pools.
    surplus: Credits,
    /// The maximum surplus (in credits) that may be implicitly donated without a `surplus_output`.
    cap: Credits,
}

impl ShieldedImplicitFeeCapExceededError {
    pub fn new(surplus: Credits, cap: Credits) -> Self {
        Self { surplus, cap }
    }

    pub fn surplus(&self) -> Credits {
        self.surplus
    }

    pub fn cap(&self) -> Credits {
        self.cap
    }
}

impl From<ShieldedImplicitFeeCapExceededError> for ConsensusError {
    fn from(err: ShieldedImplicitFeeCapExceededError) -> Self {
        Self::BasicError(BasicError::ShieldedImplicitFeeCapExceededError(err))
    }
}
