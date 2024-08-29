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
use dpp::dashcore::transaction::special_transaction::TransactionPayload::AssetUnlockPayloadType;
use dpp::dashcore::{consensus, Txid};

use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

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
            unsigned_withdrawal_transactions.len(),
        );

        let mut transaction_submission_failures = vec![];

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
                        tx_id = transaction.txid().to_hex(),
                        index,
                        "Successfully broadcasted withdrawal transaction with index {}",
                        index
                    );
                }
                // Ignore errors that can happen during blockchain synchronization.
                // They will be logged with dashcore_rpc
                Err(CoreRPCError::JsonRpc(jsonrpc::error::Error::Rpc(e)))
                    if e.code == CORE_RPC_TX_ALREADY_IN_CHAIN
                        || e.message == CORE_RPC_ERROR_ASSET_UNLOCK_NO_ACTIVE_QUORUM
                        || e.message == CORE_RPC_ERROR_ASSET_UNLOCK_EXPIRED =>
                {
                    // These will never work again
                }
                // Errors that can happen if we created invalid tx or Core isn't responding
                Err(e) => {
                    tracing::warn!(
                        tx_id = transaction.txid().to_string(),
                        index,
                        "Failed to broadcast asset unlock transaction {}: {}",
                        index,
                        e
                    );
                    // These errors might allow the state transition to be broadcast in the future
                    transaction_submission_failures.push((transaction.txid(), tx_bytes));
                }
            }
        }

        if let Some(ref rejections_path) = self.config.rejections_path {
            store_transaction_failures(transaction_submission_failures, rejections_path)
                .map_err(|e| Error::Execution(e.into()))?;
        }

        Ok(())
    }
}

// Function to handle the storage of transaction submission failures
fn store_transaction_failures(
    failures: Vec<(Txid, Vec<u8>)>,
    dir_path: &Path,
) -> std::io::Result<()> {
    if failures.is_empty() {
        return Ok(());
    }

    tracing::trace!(
        "Store {} Asset Unlock transaction submission failures in {}",
        failures.len(),
        dir_path.display()
    );

    // Ensure the directory exists
    fs::create_dir_all(dir_path).map_err(|e| {
        std::io::Error::new(
            e.kind(),
            format!("cannot create dir {}: {}", dir_path.display(), e),
        )
    })?;

    // Get the current timestamp
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("expected system time to be after unix epoch time")
        .as_secs();

    for (tx_id, transaction) in failures {
        // Create the file name
        let file_name = dir_path.join(format!("tx_{}_{}.dat", timestamp, tx_id));

        // Write the bytes to the file
        let mut file = File::create(&file_name).map_err(|e| {
            std::io::Error::new(
                e.kind(),
                format!("cannot create file {}: {}", file_name.display(), e),
            )
        })?;
        file.write_all(&transaction).map_err(|e| {
            std::io::Error::new(
                e.kind(),
                format!("cannot write to file {}: {}", file_name.display(), e),
            )
        })?;
    }

    Ok(())
}
