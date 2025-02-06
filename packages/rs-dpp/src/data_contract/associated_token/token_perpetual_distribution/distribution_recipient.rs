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
    pub fn simple_resolve(
        &self,
        contract_owner_id: Identifier,
    ) -> TokenDistributionResolvedRecipient {
        match self {
            TokenDistributionRecipient::ContractOwner => {
                TokenDistributionResolvedRecipient::ContractOwnerIdentity(contract_owner_id)
            }
            TokenDistributionRecipient::Identity(identity) => {
                TokenDistributionResolvedRecipient::Identity(*identity)
            }
            TokenDistributionRecipient::EvonodesByParticipation => {
                TokenDistributionResolvedRecipient::ResolvedEvonodesByParticipation(vec![])
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
