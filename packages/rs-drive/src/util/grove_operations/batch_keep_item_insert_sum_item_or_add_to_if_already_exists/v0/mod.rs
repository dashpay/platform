use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchInsertApplyType;
use dpp::fee::Credits;
use dpp::version::drive_versions::DriveVersion;
use dpp::ProtocolError;
use grovedb::{Element, TransactionArg};

impl Drive {
    /// Version 0 implementation of the "insert sum item or add to it if the item already exists" operation.
    /// This operation either inserts a new sum item at the given path and key or adds the value to the existing sum item.
    ///
    /// # Parameters
    /// * `path_key_element_info`: Information about the path, key, and element.
    /// * `apply_type`: The apply type for the operation.
    /// * `transaction`: The transaction argument for the operation.
    /// * `drive_operations`: The list of drive operations to append to.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(())` if the operation was successful.
    /// * `Err(DriveError::CorruptedCodeExecution)` if the operation is not supported.
    pub(super) fn batch_keep_item_insert_sum_item_or_add_to_if_already_exists_v0<D>(
        &self,
        path: &[Vec<u8>],
        key: &[u8],
        amount_to_add: Credits,
        default_item: D,
        apply_type: BatchInsertApplyType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error>
    where
        D: Into<Vec<u8>>,
    {
        // Check if the sum item already exists
        let existing_element = self.grove_get_raw_optional(
            path.into(),
            key,
            apply_type.to_direct_query_type(),
            transaction,
            drive_operations,
            drive_version,
        )?;

        if let Some(Element::ItemWithSumItem(nonce, existing_value, flags)) = existing_element {
            if amount_to_add > i64::MAX as u64 {
                return Err(ProtocolError::Overflow("amount to add over i64").into());
            }

            // Add to the existing sum item
            let updated_value = existing_value
                .checked_add(amount_to_add as i64)
                .ok_or(ProtocolError::Overflow("overflow when adding to sum item"))?;
            drive_operations.push(LowLevelDriveOperation::replace_for_known_path_key_element(
                path.to_vec(),
                key.to_vec(),
                Element::new_item_with_sum_item_with_flags(nonce, updated_value, flags),
            ));
        } else if existing_element.is_some() {
            return Err(Error::Drive(DriveError::CorruptedElementType(
                "expected item with sum item element type",
            )));
        } else {
            if amount_to_add > i64::MAX as u64 {
                return Err(ProtocolError::Overflow("amount to add over i64").into());
            }
            // Insert as a new sum item
            drive_operations.push(LowLevelDriveOperation::insert_for_known_path_key_element(
                path.to_vec(),
                key.to_vec(),
                Element::new_item_with_sum_item(default_item.into(), amount_to_add as i64),
            ));
        }
        Ok(())
    }
}
