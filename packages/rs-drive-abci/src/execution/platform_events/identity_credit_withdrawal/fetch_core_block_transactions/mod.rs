use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;

mod v0;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Fetch Core transactions by range of Core heights. This function is a version handler that
    /// directs to specific version implementations of the `fetch_core_block_transactions` function.
    ///
    /// # Arguments
    ///
    /// * `last_synced_core_height` - A u32 value representing the last synced core height.
    /// * `core_chain_locked_height` - A u32 value representing the core chain locked height.
    /// * `platform_version` - A `PlatformVersion` reference that dictates which version of
    ///   the method to call.
    ///
    /// # Returns
    ///
    /// * `Result<Vec<String>, Error>` - Returns a Vector of transaction hashes if found, otherwise returns an `Error`.
    pub(in crate::execution::platform_events::identity_credit_withdrawal) fn fetch_core_block_transactions(
        &self,
        last_synced_core_height: u32,
        core_chain_locked_height: u32,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<String>, Error> {
        match platform_version
            .drive_abci
            .methods
            .identity_credit_withdrawal
            .fetch_core_block_transactions
        {
            0 => self.fetch_core_block_transactions_v0(
                last_synced_core_height,
                core_chain_locked_height,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "fetch_core_block_transactions".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
