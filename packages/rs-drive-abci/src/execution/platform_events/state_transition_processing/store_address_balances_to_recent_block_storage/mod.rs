mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::address_funds::PlatformAddress;
use dpp::balances::credits::CreditOperation;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;
use std::collections::BTreeMap;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Stores address balance changes to recent block storage for sync purposes.
    ///
    /// This function takes a map of address balance changes and stores them
    /// serialized in the SavedBlockTransactions tree, keyed by block height.
    ///
    /// # Arguments
    ///
    /// * `address_balances` - A map of platform addresses to their credit operations
    /// * `block_info` - Information about the current block
    /// * `transaction` - The database transaction
    /// * `platform_version` - The platform version
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - Success or an error if the operation fails
    ///
    pub(in crate::execution) fn store_address_balances_to_recent_block_storage(
        &self,
        address_balances: &BTreeMap<PlatformAddress, CreditOperation>,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .state_transition_processing
            .store_address_balances_to_recent_block_storage
        {
            0 => self.store_address_balances_to_recent_block_storage_v0(
                address_balances,
                block_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "store_address_balances_to_recent_block_storage".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
