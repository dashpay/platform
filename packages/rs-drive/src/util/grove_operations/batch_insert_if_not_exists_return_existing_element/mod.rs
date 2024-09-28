mod v0;

use crate::util::grove_operations::BatchInsertApplyType;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::object_size_info::PathKeyElementInfo;

use dpp::version::drive_versions::DriveVersion;
use grovedb::{Element, TransactionArg};

impl Drive {
    /// Pushes an "insert element if the path key does not yet exist" operation to `drive_operations`
    /// and returns the existing element if it already exists.
    ///
    /// This function attempts to insert a new element into the database at a specified path and key.
    /// If an element already exists at the given path and key, it returns the existing element without inserting the new one.
    /// If the element does not exist, it inserts the new element and returns `Ok(None)`.
    ///
    /// The function dynamically selects the appropriate implementation version based on the provided `drive_version`.
    ///
    /// # Parameters
    ///
    /// * `path_key_element_info`: Information about the path, key, and element to be inserted.
    ///   - Supports various configurations including direct references, owned elements, fixed-size keys, and estimated sizes.
    /// * `apply_type`: Defines whether the operation is stateless or stateful, influencing how the insertion is handled.
    /// * `transaction`: The transaction context in which the operation will be executed.
    /// * `drive_operations`: A mutable reference to the list of drive operations where this operation will be appended.
    /// * `drive_version`: The version of the drive being used, determining which function version to execute.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(Element))`: If an element already exists at the specified path and key, it returns the existing element.
    /// * `Ok(None)`: If the element was successfully inserted because it did not exist before.
    /// * `Err(Error)`: Returns an error if:
    ///   - The drive version provided is not supported (`Error::Drive(DriveError::UnknownVersionMismatch)`).
    ///   - The operation is not supported in the current state due to invalid configurations or unsupported features (`Error::Drive(DriveError::NotSupportedPrivate)`).
    ///
    /// # Errors
    ///
    /// * `Error::Drive(DriveError::UnknownVersionMismatch)`: If the provided drive version is not supported by this function.
    /// * `Error::Drive(DriveError::NotSupportedPrivate)`: If the function encounters unsupported configurations such as unknown element sizes for batch operations.
    ///
    pub fn batch_insert_if_not_exists_return_existing_element<const N: usize>(
        &self,
        path_key_element_info: PathKeyElementInfo<N>,
        apply_type: BatchInsertApplyType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<Option<Element>, Error> {
        match drive_version.grove_methods.batch.batch_insert_if_not_exists {
            0 => self.batch_insert_if_not_exists_return_existing_element_v0(
                path_key_element_info,
                apply_type,
                transaction,
                drive_operations,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "batch_insert_if_not_exists_return_existing_element".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
