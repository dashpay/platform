use crate::platform_types::platform_state::platform_state_for_saving::v0::PlatformStateForSavingV0;
use crate::platform_types::platform_state::platform_state_for_saving::v1::PlatformStateForSavingV1;
use crate::platform_types::platform_state::platform_state_for_saving::v2::PlatformStateForSavingV2;
use bincode::Encode;
use derive_more::From;
use dpp::platform_serialization::de::Decode;

/// Structure 0 as first shipped.
pub mod v0;
/// Structure 0 with the previous fee versions.
pub mod v1;
/// Structure 1: masternodes and validator sets as entries beside the record.
pub mod v2;

/// Platform state
#[derive(Clone, Debug, Encode, Decode, From)]
pub enum PlatformStateForSaving {
    /// Version 0
    V0(PlatformStateForSavingV0),
    /// Version 1
    V1(PlatformStateForSavingV1),
    /// Version 2: the masternode list and the validator sets live as one aux
    /// entry each next to the record, not inside it.
    V2(PlatformStateForSavingV2),
}
