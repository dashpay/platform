use derive_more::From;
use serde::{Deserialize, Serialize};
use crate::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
use crate::identity::state_transition::asset_lock_proof::{Decode, Encode};

mod v0;

#[derive(Serialize, Deserialize, Encode, Decode, Debug, Clone, PartialEq, Eq, From)]
#[serde(tag = "$format_version")]
pub enum TokenConfiguration {
    #[serde(rename = "0")]
    V0(TokenConfigurationV0)
}