use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use crate::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient;

#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq)]
pub enum DistributionType {
    PreProgrammed,
    Perpetual,
}

#[derive(
    Serialize,
    Deserialize,
    Decode,
    Encode,
    PlatformSerialize,
    PlatformDeserialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
)]
#[platform_serialize(unversioned)]
pub struct TokenDistributionKey {
    pub token_id: Identifier,
    pub recipient: TokenDistributionRecipient,
    pub distribution_type: DistributionType,
}
