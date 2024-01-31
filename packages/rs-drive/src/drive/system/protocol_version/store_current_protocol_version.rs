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

impl Drive {
    /// Store the current protocol version in grovedb storage
    ///
    /// !!!DON'T CHANGE!!!!
    /// This function should never be changed !!! since it must always be compatible
    /// with fetch_current_protocol_version which is should never be changed.
    pub fn store_current_protocol_version(
        &self,
        protocol_version: ProtocolVersion,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        let mut batch_operations = vec![];

        self.set_current_protocol_version_operations(
            protocol_version,
            &mut batch_operations,
            drive_version,
        )?;

        self.apply_batch_low_level_drive_operations(
            None,
            transaction,
            batch_operations,
            &mut vec![],
            drive_version,
        )
    }

    /// Sets the current protocol version operations to batch
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
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        self.batch_insert(
            PathKeyElementInfo::PathFixedSizeKeyRefElement((
                misc_path(),
                PROTOCOL_VERSION_STORAGE_KEY,
                Element::new_item(protocol_version.encode_var_vec()),
            )),
            drive_operations,
            drive_version,
        )
    }
}
