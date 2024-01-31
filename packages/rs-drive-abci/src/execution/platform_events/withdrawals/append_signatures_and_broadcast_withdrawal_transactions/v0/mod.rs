use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::withdrawal::unsigned_withdrawal_txs::v0::UnsignedWithdrawalTxs;
use crate::rpc::core::CoreRPCLike;
use dpp::dashcore::bls_sig_utils::BLSSignature;
use dpp::dashcore::consensus::Encodable;
use dpp::dashcore::transaction::special_transaction::TransactionPayload::AssetUnlockPayloadType;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    pub(super) fn append_signatures_and_broadcast_withdrawal_transactions_v0(
        &self,
        unsigned_withdrawal_transactions: UnsignedWithdrawalTxs,
        mut signatures: Vec<BLSSignature>,
    ) -> Result<(), Error> {
        if unsigned_withdrawal_transactions.is_empty() {
            return Ok(());
        }

        tracing::debug!(
            "Broadcasting {} withdrawal transactions",
            unsigned_withdrawal_transactions.len()
        );

        for (i, mut tx) in unsigned_withdrawal_transactions.into_iter().enumerate() {
            let Some(AssetUnlockPayloadType(mut payload)) = tx.special_transaction_payload else {
                return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "withdrawal transaction payload must be AssetUnlockPayloadType",
                )));
            };

            payload.quorum_sig = signatures.remove(i);

            tx.special_transaction_payload = Some(AssetUnlockPayloadType(payload));

            let mut tx_bytes = vec![];
            tx.consensus_encode(&mut tx_bytes).unwrap();

            match self.core_rpc.send_raw_transaction(&tx_bytes) {
                Ok(_) => {
                    tracing::trace!(
                        "[Withdrawals] Broadcasted asset unlock tx: {}",
                        hex::encode(tx_bytes)
                    );
                }
                // TODO: Handle errors?
                Err(e) => {
                    tracing::error!("[Withdrawals] Failed to broadcast asset unlock tx: {}", e);
                }
            }
        }

        Ok(())
    }
}
