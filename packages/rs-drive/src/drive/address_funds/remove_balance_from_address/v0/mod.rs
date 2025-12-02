use crate::drive::Drive;
use crate::drive::RootTree;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use grovedb::{Element, TransactionArg};
use platform_version::version::PlatformVersion;

impl Drive {
    /// Version 0 implementation of removing balance from an address.
    /// This operation subtracts the specified amount from the address's current balance.
    /// The nonce stays the same. If the amount to remove exceeds the current balance,
    /// the balance is clamped to 0.
    ///
    /// # Parameters
    /// * `address`: The platform address
    /// * `amount_to_remove`: The balance amount to subtract
    /// * `drive_operations`: The list of drive operations to append to.
    /// * `transaction`: A `TransactionArg` object representing the database transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    /// * `Ok(Credits)` - The amount that was actually removed from the balance.
    /// * `Err(Error)` if the operation fails.
    pub(super) fn remove_balance_from_address_v0(
        &self,
        address: PlatformAddress,
        amount_to_remove: Credits,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, Error> {
        let path: [Vec<u8>; 1] = [vec![RootTree::AddressBalances as u8]];
        let key = address.to_bytes();

        // Fetch the existing element
        let existing_element = self.grove_get_raw_optional(
            (&path as &[Vec<u8>]).into(),
            key.as_slice(),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            drive_operations,
            &platform_version.drive,
        )?;

        match existing_element {
            Some(Element::ItemWithSumItem(nonce, existing_value, flags)) => {
                // existing_value is i64 representing the balance (should be non-negative)
                let current_balance = if existing_value < 0 {
                    0u64
                } else {
                    existing_value as u64
                };

                // Calculate actual amount to remove (clamped to current balance)
                let actual_removed = amount_to_remove.min(current_balance);
                let new_balance = current_balance.saturating_sub(actual_removed);

                drive_operations.push(LowLevelDriveOperation::replace_for_known_path_key_element(
                    path.to_vec(),
                    key,
                    Element::new_item_with_sum_item_with_flags(nonce, new_balance as i64, flags),
                ));

                Ok(actual_removed)
            }
            Some(_) => Err(Error::Drive(DriveError::CorruptedElementType(
                "expected item with sum item element type",
            ))),
            None => {
                // Address doesn't exist, nothing to remove
                Ok(0)
            }
        }
    }
}
