use dashcore::{
    blockdata::transaction::special_transaction::asset_unlock::request_info::AssetUnlockRequestInfo,
    hashes::Hash, QuorumHash,
};
use rs_drive::query::TransactionArg;

use crate::{
    error::{execution::ExecutionError, Error},
    platform::Platform,
};

const WITHDRAWAL_TRANSACTIONS_QUERY_LIMIT: u16 = 16;

impl Platform {
    /// Prepares a list of an unsigned withdrawal transaction bytes
    pub fn fetch_and_prepare_unsigned_withdrawal_transactions(
        &self,
        block_height: u32,
        validator_set_quorum_hash: [u8; 32],
        transaction: TransactionArg,
    ) -> Result<Vec<Vec<u8>>, Error> {
        // Get 16 latest withdrawal transactions from the queue
        let withdrawal_transactions = self
            .drive
            .dequeue_withdrawal_transactions(WITHDRAWAL_TRANSACTIONS_QUERY_LIMIT, transaction)?;

        // Appending request_height and quorum_hash to withdrwal transaction
        // and pass it to JS Drive for singing and broadcasting
        withdrawal_transactions
            .into_iter()
            .map(|(_, bytes)| {
                let request_info = AssetUnlockRequestInfo {
                    request_height: block_height,
                    quorum_hash: QuorumHash::hash(&validator_set_quorum_hash),
                };

                let mut bytes_buffer = vec![];

                request_info
                    .consensus_append_to_base_encode(bytes, &mut bytes_buffer)
                    .map_err(|_| {
                        Error::Execution(ExecutionError::CorruptedCodeExecution(
                            "could not add aditional request info to asset unlock transaction",
                        ))
                    })?;

                Ok(bytes_buffer)
            })
            .collect::<Result<Vec<Vec<u8>>, Error>>()
    }
}
