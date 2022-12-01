use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Asset lock output ${output_index} is not a valid standard OP_RETURN output")]
pub struct InvalidIdentityAssetLockTransactionOutputError {
    output_index: usize,
}

impl InvalidIdentityAssetLockTransactionOutputError {
    pub fn new(output_index: usize) -> Self {
        Self { output_index }
    }

    pub fn output_index(&self) -> usize {
        self.output_index
    }
}
