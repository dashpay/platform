use crate::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
use crate::identity::state_transition::asset_lock_proof::{Decode, Encode};
use derive_more::From;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

mod methods;
mod v0;

#[derive(Serialize, Deserialize, Encode, Decode, Debug, Clone, PartialEq, Eq, From)]
#[serde(tag = "$format_version")]
pub enum TokenConfiguration {
    #[serde(rename = "0")]
    V0(TokenConfigurationV0),
}

impl TokenConfiguration {
    pub fn as_cow_v0(&self) -> Cow<TokenConfigurationV0> {
        match self {
            TokenConfiguration::V0(v0) => Cow::Borrowed(v0),
        }
    }
}