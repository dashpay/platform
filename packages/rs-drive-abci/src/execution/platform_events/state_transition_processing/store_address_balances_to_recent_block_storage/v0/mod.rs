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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::epoch::Epoch;
    use std::collections::BTreeMap;

    /// Calling `store_address_balances_to_recent_block_storage_v0` with an
    /// empty balance map must be a no-op that succeeds. This covers the
    /// simplest path through the thin wrapper and proves it doesn't
    /// accidentally require any particular pre-existing drive structure.
    #[test]
    fn v0_empty_map_returns_ok() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let transaction = platform.drive.grove.start_transaction();

        let block_info = BlockInfo {
            time_ms: 1_000_000,
            height: 42,
            core_height: 10,
            epoch: Epoch::default(),
        };

        let empty_map: BTreeMap<PlatformAddress, CreditOperation> = BTreeMap::new();

        platform
            .store_address_balances_to_recent_block_storage_v0(
                &empty_map,
                &block_info,
                &transaction,
                platform_version,
            )
            .expect("storing an empty map must succeed");
    }
}
