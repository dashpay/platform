use thiserror::Error;

use crate::DPPError;

#[derive(Error, Debug, Clone, PartialEq, Eq, Default)]
#[error("Asset Lock transaction output not found")]
pub struct AssetLockOutputNotFoundError {}

impl AssetLockOutputNotFoundError {
    pub fn new() -> Self {
        Self::default()
    }
}

impl From<AssetLockOutputNotFoundError> for DPPError {
    fn from(error: AssetLockOutputNotFoundError) -> Self {
        Self::AssetLockOutputNotFoundError(error)
    }
}
