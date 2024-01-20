use crate::drive::grove_operations::BatchInsertApplyType;
use crate::drive::object_size_info::PathKeyElementInfo;
use crate::drive::system::misc_path;
use crate::drive::system::misc_tree_constants::PROTOCOL_VERSION_STORAGE_KEY;
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::util::deserializer::ProtocolVersion;
use dpp::version::drive_versions::DriveVersion;
use grovedb::{Element, TransactionArg};
use integer_encoding::VarInt;

///!!!DON'T CHANGE!!!!
impl Drive {
    /// Sets the current protocol version
    ///
    /// !!!DON'T CHANGE!!!!
    /// This function should never be changed !!! since it must always be compatible
    /// with fetch_current_protocol_version which is should never be changed.
    ///
    /// # Arguments
    ///
    /// * `protocol_version` - A `ProtocolVersion` object representing the current protocol version.
    /// * `transaction` - A `TransactionArg` object representing the transaction.
    /// * `drive_operations` - A mutable reference to a vector of `LowLevelDriveOperation` objects.
    /// * `drive_version` - A `DriveVersion` object representing the version of the Drive.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - If successful, returns an `Ok(())`. If an error occurs during the operation, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function will return an error if the version of the Drive is unknown.
    pub fn set_current_protocol_version_operations(
        &self,
        protocol_version: ProtocolVersion,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        self.batch_insert_if_changed_value(
            PathKeyElementInfo::PathFixedSizeKeyRefElement((
                misc_path(),
                PROTOCOL_VERSION_STORAGE_KEY,
                Element::new_item(protocol_version.encode_var_vec()),
            )),
            BatchInsertApplyType::StatefulBatchInsert,
            transaction,
            drive_operations,
            drive_version,
        )?;
        Ok(())
    }
}
