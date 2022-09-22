use std::sync::Arc;

use dashcore::hashes::hex::ToHex;
use dashcore::psbt::serialize::Deserialize;
use dashcore::{OutPoint, Transaction, TxOut};

use crate::identity::errors::{AssetLockOutputNotFoundError, AssetLockTransactionIsNotFoundError};
use crate::identity::state_transition::asset_lock_proof::AssetLockProof;
use crate::state_repository::StateRepositoryLike;
use crate::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use crate::DPPError;

pub struct AssetLockTransactionOutputFetcher<SR: StateRepositoryLike> {
    state_repository: Arc<SR>,
}

pub type ExecutionContext = String;

impl<SR: StateRepositoryLike> AssetLockTransactionOutputFetcher<SR> {
    pub fn new(state_repository: Arc<SR>) -> Self {
        Self { state_repository }
    }

    pub async fn fetch(
        &self,
        asset_lock_proof: &AssetLockProof,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<TxOut, DPPError> {
        match asset_lock_proof {
            AssetLockProof::Instant(asset_lock_proof) => asset_lock_proof
                .output()
                .ok_or_else(|| DPPError::from(AssetLockOutputNotFoundError::new()))
                .cloned(),
            AssetLockProof::Chain(asset_lock_proof) => {
                let out_point_buffer = *asset_lock_proof.out_point();
                let out_point = OutPoint::from(out_point_buffer);

                let output_index = out_point.vout as usize;
                let transaction_hash = out_point.txid;

                if let Some(raw_transaction) = self
                    .state_repository
                    .fetch_transaction::<Vec<u8>>(&transaction_hash.to_hex(), execution_context)
                    .await
                    .map_err(|_| DPPError::InvalidAssetLockTransaction)?
                {
                    let transaction = Transaction::deserialize(&raw_transaction)
                        .map_err(|_| DPPError::InvalidAssetLockTransaction)?;
                    transaction
                        .output
                        .get(output_index)
                        .ok_or_else(|| AssetLockOutputNotFoundError::new().into())
                        .cloned()
                } else {
                    Err(DPPError::from(AssetLockTransactionIsNotFoundError::new(
                        transaction_hash,
                    )))
                }
            }
        }
    }
}
