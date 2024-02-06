use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::withdrawal::unsigned_withdrawal_txs::v0::UnsignedWithdrawalTxs;
use crate::rpc::core::{
    CoreRPCLike, CORE_RPC_ERROR_ASSET_UNLOCK_EXPIRED, CORE_RPC_ERROR_ASSET_UNLOCK_NO_ACTIVE_QUORUM,
    CORE_RPC_TX_ALREADY_IN_CHAIN,
};
use dashcore_rpc::jsonrpc;
use dashcore_rpc::Error as CoreRPCError;
use dpp::dashcore::bls_sig_utils::BLSSignature;
use dpp::dashcore::consensus;
use dpp::dashcore::transaction::special_transaction::TransactionPayload::AssetUnlockPayloadType;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    pub(super) fn append_signatures_and_broadcast_withdrawal_transactions_v0(
        &self,
        unsigned_withdrawal_transactions: UnsignedWithdrawalTxs,
        signatures: Vec<BLSSignature>,
    ) -> Result<(), Error> {
        if unsigned_withdrawal_transactions.is_empty() {
            return Ok(());
        }

        if unsigned_withdrawal_transactions.len() != signatures.len() {
            return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                "number of signatures must match number of withdrawal transactions",
            )));
        }

        tracing::debug!(
            "Broadcasting {} withdrawal transactions",
            unsigned_withdrawal_transactions.len()
        );

        for (mut transaction, signature) in
            unsigned_withdrawal_transactions.into_iter().zip(signatures)
        {
            let Some(AssetUnlockPayloadType(mut payload)) = transaction.special_transaction_payload
            else {
                return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "withdrawal transaction payload must be AssetUnlockPayloadType",
                )));
            };

            payload.quorum_sig = signature;

            let index = payload.base.index;

            transaction.special_transaction_payload = Some(AssetUnlockPayloadType(payload));

            let tx_bytes = consensus::serialize(&transaction);

            // TODO: We need to broadcast all or none of the transactions (in case of error)
            //  will be fixed in upcoming PR
            match self.core_rpc.send_raw_transaction(&tx_bytes) {
                Ok(_) => {
                    tracing::debug!(
                        tx_id = transaction.txid().to_string(),
                        index,
                        "Successfully broadcasted asset unlock transaction {}",
                        index
                    );
                }
                // Ignore errors that can happen during blockchain catching.
                // They will be logged with dashcore_rpc
                Err(CoreRPCError::JsonRpc(jsonrpc::error::Error::Rpc(e)))
                    if e.code == CORE_RPC_TX_ALREADY_IN_CHAIN
                        || e.message == CORE_RPC_ERROR_ASSET_UNLOCK_NO_ACTIVE_QUORUM
                        || e.message == CORE_RPC_ERROR_ASSET_UNLOCK_EXPIRED => {}
                // Errors that can happen if we created invalid tx or Core isn't responding
                Err(e) => {
                    tracing::error!(
                        tx_id = transaction.txid().to_string(),
                        index,
                        "Failed to broadcast asset unlock transaction {}: {}",
                        index,
                        e
                    );

                    return Err(e.into());
                }
            }
        }

        Ok(())
    }
}
