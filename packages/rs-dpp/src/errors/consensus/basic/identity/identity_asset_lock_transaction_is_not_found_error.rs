use bincode::{Decode, Encode};
use std::fmt::{Display, Formatter};
use hex::ToHex;

use crate::consensus::basic::BasicError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::consensus::ConsensusError;

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct IdentityAssetLockTransactionIsNotFoundError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    transaction_id: [u8; 32],
}

impl Display for IdentityAssetLockTransactionIsNotFoundError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let hex = self.transaction_id.to_vec().encode_hex::<String>();
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
        Self::BasicError(BasicError::IdentityAssetLockTransactionIsNotFoundError(err))
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
