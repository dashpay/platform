use std::fmt::{Display, Formatter};

use dashcore::hashes::hex::ToHex;
use thiserror::Error;

use crate::consensus::ConsensusError;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub struct IdentityAssetLockTransactionIsNotFoundError {
    transaction_id: [u8; 32],
}

impl Display for IdentityAssetLockTransactionIsNotFoundError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let hex = self.transaction_id.to_hex();
        let message = format!("Asset Lock transaction {hex} is not found");
        f.write_str(&message)
    }
}

impl IdentityAssetLockTransactionIsNotFoundError {
    pub fn new(transaction_id: [u8; 32]) -> Self {
        Self { transaction_id }
    }

    pub fn transaction_id(&self) -> &[u8; 32] {
        &self.transaction_id
    }
}

impl From<IdentityAssetLockTransactionIsNotFoundError> for ConsensusError {
    fn from(err: IdentityAssetLockTransactionIsNotFoundError) -> Self {
        Self::IdentityAssetLockTransactionIsNotFoundError(err)
    }
}

#[cfg(test)]
mod test {
    use crate::consensus::basic::identity::IdentityAssetLockTransactionIsNotFoundError;

    #[test]
    pub fn test_message() {
        let error = IdentityAssetLockTransactionIsNotFoundError::new([1; 32]);

        println!("{}", error);
    }
}
