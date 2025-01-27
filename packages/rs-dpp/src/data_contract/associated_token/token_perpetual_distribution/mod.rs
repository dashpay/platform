use crate::data_contract::associated_token::token_perpetual_distribution::v0::TokenPerpetualDistributionV0;
use bincode::{Decode, Encode};
use derive_more::From;
use serde::{Deserialize, Serialize};
use std::fmt;

pub mod distribution_function;
pub mod v0;

#[derive(Serialize, Deserialize, Encode, Decode, Debug, Clone, PartialEq, Eq, PartialOrd, From)]
#[serde(tag = "$format_version")]
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
