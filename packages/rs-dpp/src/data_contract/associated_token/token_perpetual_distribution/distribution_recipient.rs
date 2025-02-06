use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::PlatformSerialize;
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use std::fmt;
use crate::data_contract::associated_token::token_distribution_key::{TokenDistributionType, TokenDistributionTypeWithResolvedRecipient};
use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;

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
        contract_owner_id: Identifier,
        distribution_type: TokenDistributionType,
    ) -> Result<TokenDistributionTypeWithResolvedRecipient, ProtocolError> {
        match distribution_type {
            TokenDistributionType::PreProgrammed => {
                match self {
                    TokenDistributionRecipient::ContractOwner => {
                        Ok(TokenDistributionTypeWithResolvedRecipient::PreProgrammed(contract_owner_id))
                    }
                    TokenDistributionRecipient::Identity(identity) => {
                        Ok(TokenDistributionTypeWithResolvedRecipient::PreProgrammed(*identity))
                    }
                    TokenDistributionRecipient::EvonodesByParticipation => {
                        Err(ProtocolError::NotSupported("trying to simple resolve for pre-programmed evonode distribution".to_string()))
                    }
                }
            }
            TokenDistributionType::Perpetual => {
                match self {
                    TokenDistributionRecipient::ContractOwner => {
                        Ok(TokenDistributionTypeWithResolvedRecipient::Perpetual(TokenDistributionResolvedRecipient::ContractOwnerIdentity(contract_owner_id)))
                    }
                    TokenDistributionRecipient::Identity(identity) => {
                        Ok(TokenDistributionTypeWithResolvedRecipient::Perpetual(TokenDistributionResolvedRecipient::Identity(*identity)))
                    }
                    TokenDistributionRecipient::EvonodesByParticipation => {
                        Ok(TokenDistributionTypeWithResolvedRecipient::Perpetual(TokenDistributionResolvedRecipient::ResolvedEvonodesByParticipation(vec![])))
                    }
                }
            }
        }
    }
}

pub type TokenDistributionWeight = u64;

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
    /// Distribute to many identities
    ResolvedEvonodesByParticipation(Vec<(Identifier, TokenDistributionWeight)>),
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
            TokenDistributionResolvedRecipient::ResolvedEvonodesByParticipation(identities) => {
                write!(f, "Identities[")?;
                for (i, (id, amount)) in identities.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "({}, {})", id, amount)?;
                }
                write!(f, "]")
            }
        }
    }
}
