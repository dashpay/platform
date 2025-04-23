use crate::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::From;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fmt;

pub mod accessors;
mod methods;
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
    From,
)]
#[serde(tag = "$format_version")]
#[platform_serialize(unversioned)]
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

impl fmt::Display for TokenConfiguration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenConfiguration::V0(v0) => write!(f, "{}", v0),
        }
    }
}
