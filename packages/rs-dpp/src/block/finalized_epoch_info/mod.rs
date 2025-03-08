mod getters;
pub mod v0;

use crate::block::finalized_epoch_info::v0::FinalizedEpochInfoV0;
use crate::protocol_error::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::From;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use serde::{Deserialize, Serialize};

/// Finalized Epoch information
#[derive(
    Clone,
    Debug,
    PartialEq,
    Serialize,
    Deserialize,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserialize,
    From,
)]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
pub enum FinalizedEpochInfo {
    V0(FinalizedEpochInfoV0),
}
