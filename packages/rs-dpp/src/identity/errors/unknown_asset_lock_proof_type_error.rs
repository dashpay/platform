use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("'Unknown Asset lock proof type'")]
pub struct UnknownAssetLockProofTypeError {
    asset_lock_type: Option<u8>,
}

impl UnknownAssetLockProofTypeError {
    pub fn new(asset_lock_type: Option<u8>) -> Self {
        Self { asset_lock_type }
    }

    pub fn asset_lock_type(&self) -> Option<u8> {
        self.asset_lock_type
    }
}
