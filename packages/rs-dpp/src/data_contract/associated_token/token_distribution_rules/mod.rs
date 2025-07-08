use bincode::{Decode, Encode};
use derive_more::From;
use serde::{Deserialize, Serialize};

pub mod accessors;
pub mod v0;

#[derive(Serialize, Deserialize, Encode, Decode, Debug, Clone, PartialEq, Eq, From)]
#[serde(tag = "$format_version")]
pub enum TokenDistributionRules {
    #[serde(rename = "0")]
    V0(TokenDistributionRulesV0),
}

use crate::data_contract::associated_token::token_distribution_rules::v0::TokenDistributionRulesV0;
use std::fmt;

impl fmt::Display for TokenDistributionRules {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenDistributionRules::V0(v0) => {
                write!(f, "{}", v0) //just pass through
            }
        }
    }
}
