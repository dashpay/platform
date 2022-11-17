use anyhow::{anyhow, Result};
use dashcore::{
    blockdata::transaction::special_transaction::asset_unlock::unqualified_asset_unlock::{
        AssetUnlockBasePayload, AssetUnlockBaseTransactionInfo,
    },
    consensus::Encodable,
    Script, TxOut,
};
use lazy_static::__Deref;

use crate::{
    identity::convert_credits_to_satoshi, prelude::Identity, state_repository::StateRepositoryLike,
    state_transition::StateTransitionLike,
};

use super::IdentityCreditWithdrawalTransition;

pub struct ApplyIdentityCreditWithdrawalTransition<SR>
where
    SR: StateRepositoryLike,
{
    state_repository: SR,
}

impl<SR> ApplyIdentityCreditWithdrawalTransition<SR>
where
    SR: StateRepositoryLike,
{
    pub fn new(state_repository: SR) -> Self {
        Self { state_repository }
    }

    pub async fn apply_identity_credit_withdrawal_transition(
        &self,
        state_transition: &IdentityCreditWithdrawalTransition,
    ) -> Result<()> {
        let latest_withdrawal_index = self
            .state_repository
            .fetch_latest_withdrawal_transaction_index()
            .await?;

        let output_script: Script = state_transition.output_script.deref().clone();

        let tx_out = TxOut {
            value: convert_credits_to_satoshi(state_transition.amount),
            script_pubkey: output_script,
        };

        let withdrawal_transaction = AssetUnlockBaseTransactionInfo {
            version: 1,
            lock_time: 0,
            output: vec![tx_out],
            base_payload: AssetUnlockBasePayload {
                version: 1,
                index: latest_withdrawal_index + 1,
                fee: state_transition.core_fee,
            },
        };

        let mut transaction_buffer: Vec<u8> = vec![];

        withdrawal_transaction
            .consensus_encode(&mut transaction_buffer)
            .map_err(|e| anyhow!(e))?;

        self.state_repository
            .enqueue_withdrawal_transaction(latest_withdrawal_index, transaction_buffer)
            .await?;

        let maybe_existing_identity: Option<Identity> = self
            .state_repository
            .fetch_identity(
                &state_transition.identity_id,
                state_transition.get_execution_context(),
            )
            .await?;

        let mut existing_identity =
            maybe_existing_identity.ok_or_else(|| anyhow!("Identity not found"))?;

        existing_identity.reduce_balance(state_transition.amount);

        // TODO: we need to be able to batch state repository operations
        self.state_repository
            .update_identity(&existing_identity, state_transition.get_execution_context())
            .await
    }
}
