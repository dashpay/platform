use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Asset Lock Transaction Output with index ${output_index} not found")]
pub struct IdentityAssetLockTransactionOutputNotFoundError {
    output_index: usize,
}

impl IdentityAssetLockTransactionOutputNotFoundError {
    pub fn new(output_index: usize) -> Self {
        Self { output_index }
    }

    pub fn output_index(&self) -> usize {
        self.output_index
    }
}
