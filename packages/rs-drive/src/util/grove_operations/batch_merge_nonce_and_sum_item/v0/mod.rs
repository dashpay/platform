use crate::util::grove_operations::BatchInsertApplyType;
use crate::util::object_size_info::PathKeyElementInfo::{
    PathFixedSizeKeyRefElement, PathKeyElement, PathKeyElementSize, PathKeyRefElement,
    PathKeyUnknownElementSize,
};

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::CalculatedCostOperation;
use crate::util::object_size_info::PathKeyElementInfo;
use dpp::prelude::KeyOfTypeNonce;
use dpp::version::drive_versions::DriveVersion;
use dpp::ProtocolError;
use grovedb::element::SumValue;
use grovedb::{Element, GroveDb, TransactionArg};

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
    pub(crate) fn batch_merge_nonce_and_sum_item_v0<const N: usize>(
        &self,
        path: [&[u8]; N],
        key: &[u8],
        nonce: KeyOfTypeNonce,
        add_value: SumValue,
        apply_type: BatchInsertApplyType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        // Check if the sum item already exists
        let existing_element = self.grove_get_raw_optional(
            path.as_slice().into(),
            key,
            apply_type.to_direct_query_type(),
            transaction,
            drive_operations,
            drive_version,
        )?;

        if let Some(Element::ItemWithSumItem(existing_nonce_vec, existing_value, _)) =
            existing_element
        {
            let existing_nonce_bytes: [u8; 8] =
                match existing_nonce_vec.as_slice().try_into().map_err(|_| {
                    Error::Drive(DriveError::CorruptedSerialization(
                        "existing nonce must be 8 bytes for a u64".to_string(),
                    ))
                }) {
                    Ok(value) => value,
                    Err(e) => return Err(e),
                };

            let existing_nonce = KeyOfTypeNonce::from_be_bytes(existing_nonce_bytes);

            // Add to the existing sum item
            let updated_value = existing_value
                .checked_add(add_value)
                .ok_or(ProtocolError::Overflow("overflow when adding to sum item"))?;
            drive_operations.push(LowLevelDriveOperation::insert_for_known_path_key_element(
                path.into_iter().map(|p| p.to_vec()).collect(),
                key.to_vec(),
                Element::new_item_with_sum_item(existing_nonce, updated_value),
            ));
        } else if existing_element.is_some() {
            return Err(Error::Drive(DriveError::CorruptedElementType(
                "expected item with sum item element type",
            )));
        } else {
            // Insert as a new sum item
            drive_operations.push(LowLevelDriveOperation::insert_for_known_path_key_element(
                path.into_iter().map(|p| p.to_vec()).collect(),
                key.to_vec(),
                Element::new_item_with_sum_item(nonce.to_be_bytes().to_vec(), add_value),
            ));
        }
    }
}
