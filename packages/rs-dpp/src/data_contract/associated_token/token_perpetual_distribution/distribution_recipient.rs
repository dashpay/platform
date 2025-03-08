use crate::data_contract::associated_token::token_distribution_key::{
    TokenDistributionType, TokenDistributionTypeWithResolvedRecipient,
};
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::PlatformSerialize;
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(
    Serialize,
    Deserialize,
    Decode,
    Encode,
    PlatformSerialize,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Default,
)]
#[platform_serialize(unversioned)]
pub enum TokenDistributionRecipient {
    /// Distribute to the contract Owner
    #[default]
    ContractOwner,
    /// Distribute to a single identity
    Identity(Identifier),
    /// Distribute tokens by participation
    /// This distribution can only happen when choosing epoch based distribution
    EvonodesByParticipation,
}

impl TokenDistributionRecipient {
    /// Simple resolve matches the contract owner but does not try to resolve the evonodes
    pub fn simple_resolve_with_distribution_type(
        &self,
        owner_id: Identifier,
        distribution_type: TokenDistributionType,
    ) -> Result<TokenDistributionTypeWithResolvedRecipient, ProtocolError> {
        match distribution_type {
            TokenDistributionType::PreProgrammed => match self {
                TokenDistributionRecipient::ContractOwner => Ok(
                    TokenDistributionTypeWithResolvedRecipient::PreProgrammed(owner_id),
                ),
                TokenDistributionRecipient::Identity(identity) => Ok(
                    TokenDistributionTypeWithResolvedRecipient::PreProgrammed(*identity),
                ),
                TokenDistributionRecipient::EvonodesByParticipation => {
                    Err(ProtocolError::NotSupported(
                        "trying to simple resolve for pre-programmed evonode distribution"
                            .to_string(),
                    ))
                }
            },
            TokenDistributionType::Perpetual => match self {
                TokenDistributionRecipient::ContractOwner => {
                    Ok(TokenDistributionTypeWithResolvedRecipient::Perpetual(
                        TokenDistributionResolvedRecipient::ContractOwnerIdentity(owner_id),
                    ))
                }
                TokenDistributionRecipient::Identity(identity) => {
                    Ok(TokenDistributionTypeWithResolvedRecipient::Perpetual(
                        TokenDistributionResolvedRecipient::Identity(*identity),
                    ))
                }
                TokenDistributionRecipient::EvonodesByParticipation => {
                    Ok(TokenDistributionTypeWithResolvedRecipient::Perpetual(
                        TokenDistributionResolvedRecipient::Evonode(owner_id),
                    ))
                }
            },
        }
    }
}

pub type TokenDistributionWeight = u64;

// #[derive(
//     Serialize,
//     Deserialize,
//     Decode,
//     Encode,
//     Debug,
//     Clone,
//     PartialEq,
//     Eq,
//     PartialOrd,
// )]
// pub struct EpochProposedBlocks {
//     pub block_count: u64,
//     pub total_blocks: u64,
// }

#[derive(
    Serialize,
    Deserialize,
    Decode,
    Encode,
    PlatformSerialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
)]
#[platform_serialize(unversioned)]
pub enum TokenDistributionResolvedRecipient {
    /// Distribute to a single identity
    ContractOwnerIdentity(Identifier),
    /// Distribute to a single identity
    Identity(Identifier),
    /// A single Evonode recipient that should share the token reward
    Evonode(Identifier),
}

impl From<TokenDistributionResolvedRecipient> for TokenDistributionRecipient {
    fn from(value: TokenDistributionResolvedRecipient) -> Self {
        match value {
            TokenDistributionResolvedRecipient::ContractOwnerIdentity(_) => {
                TokenDistributionRecipient::ContractOwner
            }
            TokenDistributionResolvedRecipient::Identity(identifier) => {
                TokenDistributionRecipient::Identity(identifier)
            }
            TokenDistributionResolvedRecipient::Evonode(_) => {
                TokenDistributionRecipient::EvonodesByParticipation
            }
        }
    }
}

impl From<&TokenDistributionResolvedRecipient> for TokenDistributionRecipient {
    fn from(value: &TokenDistributionResolvedRecipient) -> Self {
        match value {
            TokenDistributionResolvedRecipient::ContractOwnerIdentity(_) => {
                TokenDistributionRecipient::ContractOwner
            }
            TokenDistributionResolvedRecipient::Identity(identifier) => {
                TokenDistributionRecipient::Identity(*identifier)
            }
            TokenDistributionResolvedRecipient::Evonode(_) => {
                TokenDistributionRecipient::EvonodesByParticipation
            }
        }
    }
}

impl fmt::Display for TokenDistributionRecipient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenDistributionRecipient::ContractOwner => {
                write!(f, "ContractOwner")
            }
            TokenDistributionRecipient::Identity(identifier) => {
                write!(f, "Identity({})", identifier)
            }
            TokenDistributionRecipient::EvonodesByParticipation => {
                write!(f, "EvonodesByParticipation")
            }
        }
    }
}

/// Implements `Display` for `TokenDistributionResolvedRecipient`
impl fmt::Display for TokenDistributionResolvedRecipient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenDistributionResolvedRecipient::ContractOwnerIdentity(id) => {
                write!(f, "ContractOwnerIdentity({})", id)
            }
            TokenDistributionResolvedRecipient::Identity(id) => {
                write!(f, "Identity({})", id)
            }
            TokenDistributionResolvedRecipient::Evonode(id) => {
                write!(f, "Evonode({})", id)
            }
        }
    }
}
