use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::{
    CoreRPCLike, CORE_RPC_ERROR_ASSET_UNLOCK_EXPIRED, CORE_RPC_ERROR_ASSET_UNLOCK_NO_ACTIVE_QUORUM,
    CORE_RPC_TX_ALREADY_IN_CHAIN,
};
use dashcore_rpc::jsonrpc;
use dashcore_rpc::Error as CoreRPCError;
use dpp::dashcore::bls_sig_utils::BLSSignature;
use dpp::dashcore::transaction::special_transaction::TransactionPayload::AssetUnlockPayloadType;
use dpp::dashcore::{consensus, Transaction, Txid};
use std::collections::{BTreeMap, HashMap};

use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use tenderdash_abci::proto::types::VoteExtension;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    pub(super) fn append_signatures_and_broadcast_withdrawal_transactions_v0(
        &self,
        withdrawal_transactions_with_vote_extensions: BTreeMap<&Transaction, &VoteExtension>,
    ) -> Result<(), Error> {
        if withdrawal_transactions_with_vote_extensions.is_empty() {
            return Ok(());
        }

        tracing::debug!(
            "Broadcasting {} withdrawal transactions",
            withdrawal_transactions_with_vote_extensions.len(),
        );

        let mut transaction_submission_failures = vec![];

        for (transaction_ref, vote_extension) in withdrawal_transactions_with_vote_extensions {
            // Clone the transaction to get an owned, mutable transaction
            let mut transaction = transaction_ref.clone();

            // Extract the signature from the vote extension
            let signature_bytes: [u8; 96] = vote_extension
                .signature
                .as_slice()
                .try_into()
                .map_err(|_| {
                    Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "invalid votes extension signature size",
                    ))
                })?;

            let signature = BLSSignature::from(signature_bytes);

            // Modify the transaction's payload
            if let Some(AssetUnlockPayloadType(mut payload)) =
                transaction.special_transaction_payload
            {
                // Assign the quorum signature
                payload.quorum_sig = signature;

                // Assign the modified payload back to the transaction
                transaction.special_transaction_payload = Some(AssetUnlockPayloadType(payload));
            } else {
                return Err(Error::Execution(ExecutionError::CorruptedCachedState(
                    "withdrawal transaction payload must be AssetUnlockPayloadType".to_string(),
                )));
            }

            // Serialize the transaction
            let tx_bytes = consensus::serialize(&transaction);

            // Send the transaction
            match self.core_rpc.send_raw_transaction(&tx_bytes) {
                Ok(_) => {
                    tracing::debug!(
                        tx_id = transaction.txid().to_hex(),
                        "Successfully broadcasted withdrawal transaction"
                    );
                }
                // Handle specific errors
                Err(CoreRPCError::JsonRpc(jsonrpc::error::Error::Rpc(e)))
                    if e.code == CORE_RPC_TX_ALREADY_IN_CHAIN =>
                {
                    // Transaction already in chain; no action needed
                }
                Err(CoreRPCError::JsonRpc(jsonrpc::error::Error::Rpc(e)))
                    if e.message == CORE_RPC_ERROR_ASSET_UNLOCK_NO_ACTIVE_QUORUM
                        || e.message == CORE_RPC_ERROR_ASSET_UNLOCK_EXPIRED =>
                {
                    tracing::debug!(
                        tx_id = transaction.txid().to_string(),
                        "Asset unlock is expired or has no active quorum: {}",
                        e.message
                    );
                    transaction_submission_failures.push((transaction.txid(), tx_bytes));
                }
                // Handle other errors
                Err(e) => {
                    tracing::warn!(
                        tx_id = transaction.txid().to_string(),
                        "Failed to broadcast asset unlock transaction: {}",
                        e
                    );
                    // Collect failed transactions for potential future retries
                    transaction_submission_failures.push((transaction.txid(), tx_bytes));
                }
            }
        }

        // Store transaction submission failures
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
