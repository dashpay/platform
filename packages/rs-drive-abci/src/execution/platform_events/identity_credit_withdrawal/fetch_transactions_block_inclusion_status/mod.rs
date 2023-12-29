use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;

use dpp::version::PlatformVersion;
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
    pub(in crate::execution::platform_events::identity_credit_withdrawal) fn fetch_transactions_block_inclusion_status(
        &self,
        current_chain_locked_core_height: u32,
        transaction_identifiers: Vec<[u8; 32]>,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<[u8; 32], bool>, Error> {
        match platform_version
            .drive_abci
            .methods
            .identity_credit_withdrawal
            .fetch_transactions_block_inclusion_status
        {
            0 => self.fetch_transactions_block_inclusion_status_v0(
                current_chain_locked_core_height,
                transaction_identifiers,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "fetch_transactions_block_inclusion_status".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
