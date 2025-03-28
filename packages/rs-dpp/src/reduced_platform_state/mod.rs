use crate::serialization::{PlatformDeserializable, PlatformSerializable};
use crate::ProtocolError;
use crate::{
    reduced_platform_state::v0::ReducedPlatformStateForSavingV0,
    serialization::PlatformDeserializableFromVersionedStructure,
};
use bincode::{Decode, Encode};
use platform_version::version::PlatformVersion;

pub mod v0;

/// Reduced Platform State For Saving
#[derive(Clone, Debug, Encode, Decode)]
pub enum ReducedPlatformStateForSaving {
    V0(ReducedPlatformStateForSavingV0),
}

impl PlatformDeserializableFromVersionedStructure for ReducedPlatformStateForSaving {
    fn versioned_deserialize(
        data: &[u8],
        _platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        Ok(ReducedPlatformStateForSaving::V0(
            ReducedPlatformStateForSavingV0::deserialize_from_bytes(data)?,
        ))
    }
}

impl PlatformSerializable for ReducedPlatformStateForSaving {
    type Error = crate::errors::ProtocolError;

    fn serialize_to_bytes(&self) -> Result<Vec<u8>, Self::Error> {
        let bytes = match self {
            ReducedPlatformStateForSaving::V0(v0) => v0.serialize_to_bytes()?,
        };
        Ok(bytes)
    }
}
