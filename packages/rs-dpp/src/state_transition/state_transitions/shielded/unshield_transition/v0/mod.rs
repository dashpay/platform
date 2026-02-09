mod state_transition_like;
mod state_transition_validation;
mod types;
mod version;

use crate::address_funds::PlatformAddress;
use crate::prelude::UserFeeIncrease;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserialize,
    PlatformSignable,
    PartialEq,
)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[platform_serialize(unversioned)]
#[derive(Default)]
pub struct UnshieldTransitionV0 {
    pub output_address: PlatformAddress,
    pub amount: u64,
    pub orchard_bundle: Vec<u8>,
    pub user_fee_increase: UserFeeIncrease,
}
