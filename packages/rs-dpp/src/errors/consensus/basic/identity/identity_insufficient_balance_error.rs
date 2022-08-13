use thiserror::Error;

use crate::prelude::Identifier;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Insufficient identity ${identity_id} balance ${balance}")]
pub struct IdentityInsufficientBalanceError {
    pub identity_id: Identifier,
    pub balance: u64,
}

impl IdentityInsufficientBalanceError {
    pub fn new(identity_id: Identifier, balance: u64) -> Self {
        Self {
            identity_id,
            balance,
        }
    }

    pub fn identity_id(&self) -> &Identifier {
        &self.identity_id
    }

    pub fn balance(&self) -> u64 {
        self.balance
    }
}
