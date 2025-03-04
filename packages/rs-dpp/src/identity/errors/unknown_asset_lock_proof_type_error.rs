use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("'Unknown Asset lock proof type'")]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct UnknownAssetLockProofTypeError {
    pub asset_lock_type: Option<u8>,
}

impl UnknownAssetLockProofTypeError {
    pub fn new(asset_lock_type: Option<u8>) -> Self {
        Self { asset_lock_type }
    }

    pub fn asset_lock_type(&self) -> Option<u8> {
        self.asset_lock_type
    }
}
