use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::document::Document;
use dpp::fee::Credits;
use dpp::version::PlatformVersion;
use dpp::withdrawal::{WithdrawalTransactionIndex, WithdrawalTransactionIndexAndBytes};

mod v0;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Builds a list of withdrawal transactions from the provided withdrawal documents.
    /// Each withdrawal document is converted into a Core transaction, starting from the specified index.
    /// The function encodes the transaction and updates the document with the transaction index, status,
    /// updated time, and revision.
    ///
    /// # Arguments
    ///
    /// * `documents` - A mutable reference to a vector of `Document` representing the withdrawal requests.
    /// * `start_index` - The starting index for the transaction, of type `WithdrawalTransactionIndex`.
    /// * `block_info` - A reference to the `BlockInfo`, which provides the current block's timestamp.
    /// * `platform_version` - A reference to the `PlatformVersion` that specifies the version of the platform being used.
    ///
    /// # Returns
    ///
    /// * `Result<Vec<WithdrawalTransactionIndexAndBytes>, Error>` - On success, returns a vector of tuples containing the
    ///   transaction index and the encoded transaction bytes. On failure, returns an `Error`.
    pub(in crate::execution::platform_events::withdrawals) fn build_untied_withdrawal_transactions_from_documents(
        &self,
        documents: &mut [Document],
        start_index: WithdrawalTransactionIndex,
        block_info: &BlockInfo,
        platform_version: &PlatformVersion,
    ) -> Result<(Vec<WithdrawalTransactionIndexAndBytes>, Credits), Error> {
        match platform_version
            .drive_abci
            .methods
            .withdrawals
            .build_untied_withdrawal_transactions_from_documents
        {
            0 => self.build_untied_withdrawal_transactions_from_documents_v0(
                documents,
                start_index,
                block_info,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "build_untied_withdrawal_transactions_from_documents".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
