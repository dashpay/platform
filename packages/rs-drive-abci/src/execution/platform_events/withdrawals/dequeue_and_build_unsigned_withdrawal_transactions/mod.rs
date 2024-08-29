use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;

use crate::platform_types::withdrawal::unsigned_withdrawal_txs::v0::UnsignedWithdrawalTxs;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;

use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

mod v0;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Prepares a list of an unsigned withdrawal transaction bytes. This function is a version handler that
    /// directs to specific version implementations of the `fetch_and_prepare_unsigned_withdrawal_transactions` function.
    ///
    /// # Arguments
    ///
    /// * `validator_set_quorum_hash` - A byte array.
    /// * `block_execution_context` - A `BlockExecutionContext` reference.
    /// * `transaction` - A `Transaction` reference.
    /// * `platform_version` - A `PlatformVersion` reference that dictates which version of
    ///   the method to call.
    ///
    /// # Returns
    ///
    /// * `Result<Vec<Vec<u8>>, Error>` - Returns a Vector of unsigned withdrawal transactions if found, otherwise returns an `Error`.
    pub(in crate::execution) fn dequeue_and_build_unsigned_withdrawal_transactions(
        &self,
        validator_set_quorum_hash: [u8; 32],
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<UnsignedWithdrawalTxs, Error> {
        match platform_version
            .drive_abci
            .methods
            .withdrawals
            .dequeue_and_build_unsigned_withdrawal_transactions
        {
            0 => self.dequeue_and_build_unsigned_withdrawal_transactions_v0(
                validator_set_quorum_hash,
                block_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "dequeue_and_build_unsigned_withdrawal_transactions".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
