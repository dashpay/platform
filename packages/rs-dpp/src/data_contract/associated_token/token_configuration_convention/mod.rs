use crate::data_contract::associated_token::token_configuration_convention::v0::TokenConfigurationConventionV0;
use bincode::{Decode, Encode};
use derive_more::From;
use serde::{Deserialize, Serialize};

mod accessors;
mod methods;
pub mod v0;

#[derive(Serialize, Deserialize, Encode, Decode, Debug, Clone, PartialEq, Eq, PartialOrd, From)]
#[serde(tag = "$format_version")]
pub enum TokenConfigurationConvention {
    #[serde(rename = "0")]
    V0(TokenConfigurationConventionV0),
}

use std::fmt;

impl fmt::Display for TokenConfigurationConvention {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenConfigurationConvention::V0(v0) => {
                write!(f, "{}", v0) //just pass through
            }
        }
    }
}
