use crate::drive::Drive;
use crate::drive::RootTree;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchInsertApplyType;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Version 0 implementation of adding a balance for an address.
    /// This operation directly adds the balance for the address.
    /// The nonce stays the same. If there is no address the nonce becomes 0.
    ///
    /// # Parameters
    /// * `address`: The platform address
    /// * `balance`: The balance value to set
    /// * `drive_operations`: The list of drive operations to append to.
    ///
    /// # Returns
    /// * `Ok(())` if the operation was successful.
    /// * `Err(Error)` if the operation fails.
    pub(super) fn add_balance_to_address_v0(
        &self,
        address: PlatformAddress,
        amount_to_add: Credits,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let path = vec![vec![RootTree::AddressBalances as u8]];

        let key = address.to_bytes();

        self.batch_keep_item_insert_sum_item_or_add_to_if_already_exists(
            &path,
            key.as_slice(),
            amount_to_add,
            BatchInsertApplyType::StatefulBatchInsert,
            transaction,
            drive_operations,
            &platform_version.drive,
        )
    }
}
