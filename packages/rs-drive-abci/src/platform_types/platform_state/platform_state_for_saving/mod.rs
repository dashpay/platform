use crate::platform_types::platform_state::platform_state_for_saving::v0::PlatformStateForSavingV0;
use crate::platform_types::platform_state::platform_state_for_saving::v1::PlatformStateForSavingV1;
use bincode::Encode;
use derive_more::From;
use dpp::platform_serialization::de::Decode;

pub mod v0;
pub mod v1;

/// Platform state
#[derive(Clone, Debug, Encode, Decode, From)]
pub enum PlatformStateForSaving {
    /// Version 0
    V0(PlatformStateForSavingV0),
    /// Version 1
    V1(PlatformStateForSavingV1),
}
