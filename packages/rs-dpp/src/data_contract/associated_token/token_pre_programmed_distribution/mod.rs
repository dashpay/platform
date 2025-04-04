use crate::data_contract::associated_token::token_pre_programmed_distribution::v0::TokenPreProgrammedDistributionV0;
use bincode::{Decode, Encode};
use derive_more::From;
use serde::{Deserialize, Serialize};
use std::fmt;

pub mod accessors;

pub mod v0;

#[derive(Serialize, Deserialize, Encode, Decode, Debug, Clone, PartialEq, Eq, From)]
#[serde(tag = "$format_version")]
pub enum TokenPreProgrammedDistribution {
    #[serde(rename = "0")]
    V0(TokenPreProgrammedDistributionV0),
}

impl fmt::Display for TokenPreProgrammedDistribution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenPreProgrammedDistribution::V0(v0) => {
                write!(f, "{}", v0) //just pass through
            }
        }
    }
}
