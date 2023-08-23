use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::block_execution_context::BlockExecutionContext;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;

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
    pub(in crate::execution) fn fetch_and_prepare_unsigned_withdrawal_transactions(
        &self,
        validator_set_quorum_hash: [u8; 32],
        block_execution_context: &BlockExecutionContext,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Vec<u8>>, Error> {
        match platform_version
            .drive_abci
            .methods
            .identity_credit_withdrawal
            .fetch_and_prepare_unsigned_withdrawal_transactions
        {
            0 => self.fetch_and_prepare_unsigned_withdrawal_transactions_v0(
                validator_set_quorum_hash,
                block_execution_context,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "fetch_and_prepare_unsigned_withdrawal_transactions".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
