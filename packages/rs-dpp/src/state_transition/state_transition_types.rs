use anyhow::bail;
use std::convert::TryFrom;

use serde_repr::{Deserialize_repr, Serialize_repr};

#[repr(u8)]
#[derive(Serialize_repr, Deserialize_repr, PartialEq, Eq, Clone, Copy, Debug)]
pub enum StateTransitionType {
    DataContractCreate = 0,
    DocumentsBatch = 1,
    IdentityCreate = 2,
    IdentityTopUp = 3,
    DataContractUpdate = 4,
    IdentityUpdate = 5,
    IdentityCreditWithdrawal = 6,
}

impl TryFrom<u8> for StateTransitionType {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::DataContractCreate),
            1 => Ok(Self::DocumentsBatch),
            2 => Ok(Self::IdentityCreate),
            3 => Ok(Self::IdentityTopUp),
            4 => Ok(Self::DataContractUpdate),
            5 => Ok(Self::IdentityUpdate),
            6 => Ok(Self::IdentityCreditWithdrawal),
            _ => bail!("The '{}' isn't a valid type of state transition", value),
        }
    }
}

impl std::fmt::Display for StateTransitionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::default::Default for StateTransitionType {
    fn default() -> Self {
        StateTransitionType::DataContractCreate
    }
}
