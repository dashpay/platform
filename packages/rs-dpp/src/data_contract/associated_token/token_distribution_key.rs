use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use serde::{Deserialize, Serialize};

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
    pub identity_id: Identifier,
    pub distribution_type: DistributionType,
}
