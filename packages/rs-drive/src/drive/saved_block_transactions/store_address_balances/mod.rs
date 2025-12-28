mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::address_funds::PlatformAddress;
use dpp::balances::credits::CreditOperation;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

impl Drive {
    /// Stores address balance changes for a block in the SavedBlockTransactions tree.
    ///
    /// This method serializes the address balance changes using bincode and stores
    /// them keyed by block height.
    ///
    /// # Parameters
    /// - `address_balances`: The map of address balance changes to store
    /// - `block_height`: The height of the block these changes belong to
    /// - `transaction`: The database transaction
    /// - `platform_version`: The platform version
    ///
    /// # Returns
    /// - `Ok(())` on success
    /// - `Err(Error)` if the operation fails
    pub fn store_address_balances_for_block(
        &self,
        address_balances: &BTreeMap<PlatformAddress, CreditOperation>,
        block_height: u64,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .saved_block_transactions
            .store_address_balances
        {
            0 => self.store_address_balances_for_block_v0(
                address_balances,
                block_height,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "store_address_balances_for_block".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
