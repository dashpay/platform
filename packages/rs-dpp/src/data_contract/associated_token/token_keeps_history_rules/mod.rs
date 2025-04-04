use bincode::{Decode, Encode};
use derive_more::From;
use serde::{Deserialize, Serialize};

pub mod accessors;
pub mod v0;

#[derive(Serialize, Deserialize, Encode, Decode, Debug, Clone, Copy, PartialEq, Eq, From)]
#[serde(tag = "$format_version")]
pub enum TokenKeepsHistoryRules {
    #[serde(rename = "0")]
    V0(TokenKeepsHistoryRulesV0),
}

use crate::data_contract::associated_token::token_keeps_history_rules::v0::TokenKeepsHistoryRulesV0;
use std::fmt;

impl fmt::Display for TokenKeepsHistoryRules {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKeepsHistoryRules::V0(v0) => {
                write!(f, "{}", v0) //just pass through
            }
        }
    }
}
