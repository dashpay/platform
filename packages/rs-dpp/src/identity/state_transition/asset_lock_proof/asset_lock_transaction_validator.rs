use std::sync::Arc;

use dashcore::consensus;
use dashcore::{OutPoint, Transaction};

use crate::consensus::basic::identity::{
    IdentityAssetLockTransactionOutPointAlreadyExistsError,
    IdentityAssetLockTransactionOutputNotFoundError, InvalidAssetLockTransactionOutputReturnSize,
    InvalidIdentityAssetLockTransactionError, InvalidIdentityAssetLockTransactionOutputError,
};
use crate::state_repository::StateRepositoryLike;
use crate::util::vec::vec_to_array;
use crate::validation::ValidationResult;
use crate::NonConsensusError;

#[derive(Clone, Debug)]
pub struct AssetLockTransactionResultData {
    pub public_key_hash: [u8; 20],
    pub transaction: Transaction,
}

impl Default for AssetLockTransactionResultData {
    fn default() -> Self {
        Self {
            public_key_hash: Default::default(),
            transaction: Transaction {
                version: 0,
                lock_time: 0,
                input: vec![],
                output: vec![],
            },
        }
    }
}

pub struct AssetLockTransactionValidator<SR>
where
    SR: StateRepositoryLike,
{
    state_repository: Arc<SR>,
}

impl<SR> AssetLockTransactionValidator<SR>
where
    SR: StateRepositoryLike,
{
    pub fn new(state_repository: Arc<SR>) -> Self {
        Self { state_repository }
    }

    /// raw_tx should be a js uint array
    pub async fn validate(
        &self,
        raw_tx: &[u8],
        output_index: usize,
    ) -> Result<ValidationResult<AssetLockTransactionResultData>, NonConsensusError> {
        let mut result = ValidationResult::default();

        match consensus::deserialize::<Transaction>(raw_tx) {
            Ok(transaction) => {
                let output = transaction.output.get(output_index);

                if let Some(output) = output {
                    if !output.script_pubkey.is_op_return() {
                        result.add_error(InvalidIdentityAssetLockTransactionOutputError::new(
                            output_index,
                        ));
                        return Ok(result);
                    }

                    // Slicing from 1 bytes, which is OP_RETURN, to the end of the script
                    let public_key_hash = &output.script_pubkey.as_bytes()[2..];
                    // 20 bytes is the size of ripemd160, which should be stored after the OP_RETURN
                    if public_key_hash.len() != 20 {
                        result.add_error(InvalidAssetLockTransactionOutputReturnSize::new(
                            output_index,
                        ));
                        return Ok(result);
                    }

                    let out_point = OutPoint::new(transaction.txid(), output_index as u32);
                    let out_point_buf = consensus::serialize(&out_point);

                    let is_out_point_used = self
                        .state_repository
                        .is_asset_lock_transaction_out_point_already_used(&out_point_buf)
                        .await
                        .map_err(|err| {
                            NonConsensusError::StateRepositoryFetchError(err.to_string())
                        })?;

                    if is_out_point_used {
                        result.add_error(
                            IdentityAssetLockTransactionOutPointAlreadyExistsError::new(
                                transaction.txid(),
                                output_index,
                            ),
                        );
                        return Ok(result);
                    }

                    result.set_data(AssetLockTransactionResultData {
                        public_key_hash: vec_to_array(&output.script_pubkey.as_bytes()[2..22])?,
                        transaction,
                    });

                    Ok(result)
                } else {
                    result.add_error(IdentityAssetLockTransactionOutputNotFoundError::new(
                        output_index,
                    ));
                    Ok(result)
                }
            }
            Err(err) => {
                let mut error = InvalidIdentityAssetLockTransactionError::new(err.to_string());
                error.set_validation_error(err);

                result.add_error(error);
                Ok(result)
            }
        }
    }
}
