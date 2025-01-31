use bincode::{Decode, Encode};
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq, PartialOrd)]
pub enum TokenPerpetualDistributionRecipient {
    /// Distribute to the contract Owner
    ContractOwner,
    /// Distribute to a single identity
    Identity(Identifier),
    /// Distribute tokens by participation
    /// This distribution can only happen when choosing epoch based distribution
    EvonodesByParticipation,
}

impl fmt::Display for TokenPerpetualDistributionRecipient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenPerpetualDistributionRecipient::ContractOwner => {
                write!(f, "ContractOwner")
            }
            TokenPerpetualDistributionRecipient::Identity(identifier) => {
                write!(f, "Identity({})", identifier)
            }
            TokenPerpetualDistributionRecipient::EvonodesByParticipation => {
                write!(f, "EvonodesByParticipation")
            }
        }
    }
}
