use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::dashcore_rpc::json::AssetUnlockStatus;
use dpp::version::PlatformVersion;
use dpp::withdrawal::WithdrawalTransactionIndex;
use std::collections::BTreeMap;

mod v0;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Fetch Core transactions by range of Core heights. This function is a version handler that
    /// directs to specific version implementations of the `fetch_transactions_block_inclusion_status` function.
    ///
    /// # Arguments
    ///
    /// * `current_chain_locked_core_height` - The current chain locked core height
    /// * `transactions` - A list of transactions to fetch.
    /// * `platform_version` - A `PlatformVersion` reference that dictates which version of
    ///   the method to call.
    ///
    /// # Returns
    ///
    /// * `Result<Vec<String>, Error>` - Returns a Vector of transaction hashes if found, otherwise returns an `Error`.
    pub(in crate::execution::platform_events::withdrawals) fn fetch_transactions_block_inclusion_status(
        &self,
        current_chain_locked_core_height: u32,
        withdrawal_indices: &[WithdrawalTransactionIndex],
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<WithdrawalTransactionIndex, AssetUnlockStatus>, Error> {
        match platform_version
            .drive_abci
            .methods
            .withdrawals
            .fetch_transactions_block_inclusion_status
        {
            0 => self.fetch_transactions_block_inclusion_status_v0(
                current_chain_locked_core_height,
                withdrawal_indices,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "fetch_transactions_block_inclusion_status".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
