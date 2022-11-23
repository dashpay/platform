use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("'Unknown Asset lock proof type'")]
pub struct UnknownAssetLockProofTypeError {
    asset_lock_type: u32,
}

impl UnknownAssetLockProofTypeError {
    pub fn new(asset_lock_type: u32) -> Self {
        Self { asset_lock_type }
    }

    pub fn asset_lock_type(&self) -> u32 {
        self.asset_lock_type
    }
}
