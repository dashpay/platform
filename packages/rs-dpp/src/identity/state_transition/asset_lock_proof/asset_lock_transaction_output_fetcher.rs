use std::convert::TryInto;
use std::sync::Arc;

use dashcore::hashes::hex::ToHex;
use dashcore::psbt::serialize::Deserialize;
use dashcore::{OutPoint, Transaction, TxOut};

use crate::identity::errors::{AssetLockOutputNotFoundError, AssetLockTransactionIsNotFoundError};
use crate::identity::state_transition::asset_lock_proof::AssetLockProof;
use crate::state_repository::StateRepositoryLike;
use crate::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use crate::DPPError;

pub struct AssetLockTransactionOutputFetcher<SR> {
    state_repository: Arc<SR>,
}

impl<SR: StateRepositoryLike> AssetLockTransactionOutputFetcher<SR> {
    pub fn new(state_repository: Arc<SR>) -> Self {
        Self { state_repository }
    }

    pub async fn fetch(
        &self,
        asset_lock_proof: &AssetLockProof,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<TxOut, DPPError> {
        fetch_asset_lock_transaction_output(
            self.state_repository.as_ref(),
            asset_lock_proof,
            execution_context,
        )
        .await
    }
}

pub async fn fetch_asset_lock_transaction_output(
    state_repository: &impl StateRepositoryLike,
    asset_lock_proof: &AssetLockProof,
    execution_context: &StateTransitionExecutionContext,
) -> Result<TxOut, DPPError> {
    match asset_lock_proof {
        AssetLockProof::Instant(asset_lock_proof) => asset_lock_proof
            .output()
            .ok_or_else(|| DPPError::from(AssetLockOutputNotFoundError::new()))
            .cloned(),
        AssetLockProof::Chain(asset_lock_proof) => {
            let out_point = OutPoint::from(asset_lock_proof.out_point.to_buffer());

            let output_index = out_point.vout as usize;
            let transaction_hash = out_point.txid;

            let transaction_data = state_repository
                .fetch_transaction(&transaction_hash.to_hex(), execution_context)
                .await
                .map_err(|_| DPPError::InvalidAssetLockTransaction)?;

            if execution_context.is_dry_run() {
                return Ok(TxOut {
                    value: 1000,
                    ..Default::default()
                });
            }

            let transaction_data = transaction_data
                .try_into()
                .map_err(|_| DPPError::InvalidAssetLockTransaction)?;

            if let Some(raw_transaction) = transaction_data.data {
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

#[cfg(test)]
mod test {
    use crate::state_repository::FetchTransactionResponse;
    use crate::{
        identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof,
        state_repository::MockStateRepositoryLike,
    };

    use super::*;

    #[tokio::test]
    async fn should_return_mocked_data_on_dry_run() {
        let mut state_repository_mock = MockStateRepositoryLike::new();
        let asset_lock_proof = &AssetLockProof::Chain(ChainAssetLockProof::new(0, [0u8; 36]));
        let execution_context = StateTransitionExecutionContext::default();

        state_repository_mock
            .expect_fetch_transaction()
            .return_once(|_, _| Ok(FetchTransactionResponse::default()));
        execution_context.enable_dry_run();

        let result = fetch_asset_lock_transaction_output(
            &state_repository_mock,
            asset_lock_proof,
            &execution_context,
        )
        .await
        .expect("the transaction output should be returned");

        assert_eq!(
            TxOut {
                value: 1000,
                ..Default::default()
            },
            result
        );
    }
}
