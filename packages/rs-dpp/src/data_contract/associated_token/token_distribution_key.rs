use crate::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::{TokenDistributionRecipient, TokenDistributionResolvedRecipient};
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use std::fmt;
use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
use crate::prelude::TimestampMillis;

/// Represents the type of token distribution.
///
/// - `PreProgrammed`: A scheduled distribution with predefined rules.
/// - `Perpetual`: A continuous or recurring distribution.
#[derive(
    Serialize, Deserialize, Decode, Encode, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Default,
)]
pub enum TokenDistributionType {
    /// A pre-programmed distribution scheduled for a specific time.
    #[default]
    PreProgrammed = 0,

    /// A perpetual distribution that occurs at regular intervals.
    Perpetual = 1,
}

/// Represents a token distribution with a resolved recipient.
///
/// - `PreProgrammed(Identifier)`: A predefined recipient for a scheduled distribution.
/// - `Perpetual(TokenDistributionResolvedRecipient)`: A resolved recipient for an ongoing distribution.
#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq, PartialOrd)]
pub enum TokenDistributionTypeWithResolvedRecipient {
    /// A scheduled distribution with a known recipient.
    PreProgrammed(Identifier),

    /// A perpetual distribution with a resolved recipient.
    Perpetual(TokenDistributionResolvedRecipient),
}

/// Contains information about a specific token distribution instance.
///
/// - `PreProgrammed(TimestampMillis, Identifier)`: A scheduled distribution with a timestamp and recipient.
/// - `Perpetual(RewardDistributionMoment, RewardDistributionMoment, TokenDistributionResolvedRecipient)`:
///   A perpetual distribution with previous and next distribution moments, along with the resolved recipient.
#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq, PartialOrd)]
pub enum TokenDistributionInfo {
    /// A pre-programmed token distribution set for a specific time.
    /// Contains the scheduled timestamp and the recipient’s identifier.
    PreProgrammed(TimestampMillis, Identifier),

    /// A perpetual token distribution with moment for distribution.
    /// The moment is the beginning of the perpetual distribution cycle
    /// Includes the last and next distribution times and the resolved recipient.
    Perpetual(RewardDistributionMoment, TokenDistributionResolvedRecipient),
}

impl From<TokenDistributionInfo> for TokenDistributionTypeWithResolvedRecipient {
    fn from(info: TokenDistributionInfo) -> Self {
        match info {
            TokenDistributionInfo::PreProgrammed(_, recipient) => {
                TokenDistributionTypeWithResolvedRecipient::PreProgrammed(recipient)
            }
            TokenDistributionInfo::Perpetual(_, recipient) => {
                TokenDistributionTypeWithResolvedRecipient::Perpetual(recipient)
            }
        }
    }
}

impl From<&TokenDistributionInfo> for TokenDistributionTypeWithResolvedRecipient {
    fn from(info: &TokenDistributionInfo) -> Self {
        match info {
            TokenDistributionInfo::PreProgrammed(_, recipient) => {
                TokenDistributionTypeWithResolvedRecipient::PreProgrammed(*recipient)
            }
            TokenDistributionInfo::Perpetual(_, recipient) => {
                TokenDistributionTypeWithResolvedRecipient::Perpetual(recipient.clone())
            }
        }
    }
}

impl fmt::Display for TokenDistributionType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TokenDistributionType::PreProgrammed => write!(f, "PreProgrammed"),
            TokenDistributionType::Perpetual => write!(f, "Perpetual"),
        }
    }
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
    pub distribution_type: TokenDistributionType,
}
