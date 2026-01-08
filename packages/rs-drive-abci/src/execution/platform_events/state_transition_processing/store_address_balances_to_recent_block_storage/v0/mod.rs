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
    /// Version 0 implementation of storing address balance changes to recent block storage.
    ///
    /// This method serializes the address balance map using bincode and stores it
    /// in the SavedBlockTransactions tree for sync purposes.
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
    pub(super) fn store_address_balances_to_recent_block_storage_v0(
        &self,
        address_balances: &BTreeMap<PlatformAddress, CreditOperation>,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        self.drive.store_address_balances_for_block(
            address_balances,
            block_info.height,
            block_info.time_ms,
            Some(transaction),
            platform_version,
        )?;

        Ok(())
    }
}
