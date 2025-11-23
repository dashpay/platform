mod v0;

use crate::util::grove_operations::BatchInsertApplyType;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;

use dpp::fee::Credits;
use dpp::version::drive_versions::DriveVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Adds a “sum item insert-or-add” operation to `drive_operations`.
    ///
    /// This operation attempts to insert a new [`Element::ItemWithSumItem`] at the given
    /// `path` and `key`. If an item with sum item already exists at that location, its
    /// value is **increased** by `amount_to_add` instead of inserting a new item.
    ///
    /// This is used when persistent state needs to maintain an accumulated
    /// numeric value at a specific key (e.g., balances, counters, aggregated
    /// metadata) while ensuring the operation is applied deterministically as a
    /// single atomic update.
    ///
    /// # Parameters
    ///
    /// - `path`: The GroveDB path where the item should reside.
    /// - `key`: The element key to insert or update.
    /// - `amount_to_add`: The value that should be inserted or added.
    /// - `apply_type`: Controls whether the operation is applied, estimated,
    ///    or queued for batch processing.
    /// - `transaction`: Optional transaction context for the underlying GroveDB call.
    /// - `drive_operations`: The vector of low-level drive operations this call
    ///    should append to.
    /// - `drive_version`: The versioned function selector used to dispatch to the
    ///    correct implementation.
    ///
    /// # Behavior
    ///
    /// - If the key **does not** exist, a new sum item is inserted with
    ///   value = `amount_to_add`.
    /// - If the key **does** exist and contains a sum item, the stored value is
    ///   increased by `amount_to_add`.
    /// - If the key exists but the element is **not** a sum item, an error is returned.
    ///
    /// # Returns
    ///
    /// - `Ok(())` on success.
    /// - `Err(DriveError::UnknownVersionMismatch)` if an unsupported
    ///   function version is encountered.
    /// - `Err(DriveError::CorruptedElementType)` if the existing element is not
    ///   compatible with a sum item update (indicating corrupted state).
    /// - `Err(DriveError::CorruptedCodeExecution)` if the method is not
    ///   implemented for the selected version (should not occur in production).
    pub fn batch_keep_item_insert_sum_item_or_add_to_if_already_exists(
        &self,
        path: &[Vec<u8>],
        key: &[u8],
        amount_to_add: Credits,
        apply_type: BatchInsertApplyType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version
            .grove_methods
            .batch
            .batch_insert_sum_item_or_add_to_if_already_exists
        {
            0 => self.batch_keep_item_insert_sum_item_or_add_to_if_already_exists_v0(
                path,
                key,
                amount_to_add,
                apply_type,
                transaction,
                drive_operations,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "batch_keep_item_insert_sum_item_or_add_to_if_already_exists".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
