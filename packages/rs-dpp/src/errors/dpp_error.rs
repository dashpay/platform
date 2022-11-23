use thiserror::Error;

use crate::identity::errors::{AssetLockOutputNotFoundError, AssetLockTransactionIsNotFoundError};

#[derive(Error, Debug)]
#[error("{0}")]
pub enum DPPError {
    AssetLockOutputNotFoundError(AssetLockOutputNotFoundError),
    AssetLockTransactionIsNotFoundError(AssetLockTransactionIsNotFoundError),
    #[error("Expected public key hash to be 20 bytes")]
    WrongPublicKeyHashSize,
    #[error("Expected output type to be OP_RETURN output")]
    WrongBurnOutputType,
    #[error("Invalid transaction")]
    InvalidAssetLockTransaction,
}
