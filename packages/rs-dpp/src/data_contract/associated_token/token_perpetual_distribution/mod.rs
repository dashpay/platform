use crate::data_contract::associated_token::token_perpetual_distribution::v0::TokenPerpetualDistributionV0;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::From;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use serde::{Deserialize, Serialize};
use std::fmt;

pub mod distribution_function;
pub mod distribution_recipient;
pub mod methods;
pub mod reward_distribution_moment;
pub mod reward_distribution_type;
pub mod v0;

#[derive(
    Serialize,
    Deserialize,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    From,
)]
#[serde(tag = "$format_version")]
#[platform_serialize(unversioned)]
pub enum TokenPerpetualDistribution {
    #[serde(rename = "0")]
    V0(TokenPerpetualDistributionV0),
}

impl fmt::Display for TokenPerpetualDistribution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenPerpetualDistribution::V0(v0) => {
                write!(f, "{}", v0) //just pass through
            }
        }
    }
}
