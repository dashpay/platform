use thiserror::Error;

use crate::identity::errors::{AssetLockOutputNotFoundError, AssetLockTransactionIsNotFoundError};

#[derive(Error, Debug)]
#[error("{0}")]
pub enum DPPError {
    AssetLockOutputNotFoundError(AssetLockOutputNotFoundError),
    AssetLockTransactionIsNotFoundError(AssetLockTransactionIsNotFoundError),
    #[error("expected public key hash to be 20 bytes")]
    WrongPublicKeyHashSize,
    #[error("expected output type to be OP_RETURN output")]
    WrongBurnOutputType,
    #[error("invalid transaction")]
    InvalidAssetLockTransaction,
    #[error("core message corruption {0}")]
    CoreMessageCorruption(String),
}
