use crate::reduced_platform_state::v0::ReducedPlatformStateForSavingV0;
use crate::serialization::ReducedPlatformDeserializable;
use crate::ProtocolError;
use bincode::{config, Decode, Encode};
use platform_version::version::PlatformVersion;

pub mod v0;

/// Reduced Platform State For Saving
#[derive(Clone, Debug, Encode, Decode)]
pub enum ReducedPlatformStateForSaving {
    V0(ReducedPlatformStateForSavingV0),
}

impl ReducedPlatformDeserializable for ReducedPlatformStateForSaving {
    fn versioned_deserialize(
        data: &[u8],
        _platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        let config = config::standard().with_big_endian().with_no_limit();
        let reduced_platform_state_in_save_format: ReducedPlatformStateForSaving =
            bincode::decode_from_slice(data, config)
                .map_err(|e| {
                    ProtocolError::PlatformDeserializationError(format!(
                        "unable to deserialize ReducedPlatformStateForSaving: {}",
                        e
                    ))
                })?
                .0;
        Ok(reduced_platform_state_in_save_format)
    }
}
