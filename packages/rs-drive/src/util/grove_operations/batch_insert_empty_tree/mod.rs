mod v0;

use crate::util::object_size_info::DriveKeyInfo;
use crate::util::storage_flags::StorageFlags;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::drive_versions::DriveVersion;

impl Drive {
    /// Pushes an "insert empty tree" operation to `drive_operations`.
    ///
    /// # Parameters
    /// * `path`: The path to insert an empty tree.
    /// * `key_info`: The key information of the document.
    /// * `storage_flags`: Storage options for the operation.
    /// * `drive_operations`: The vector containing low-level drive operations.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(())` if the operation was successful.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    pub fn batch_insert_empty_tree<'a, 'c, P>(
        &'a self,
        path: P,
        key_info: DriveKeyInfo<'c>,
        storage_flags: Option<&StorageFlags>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error>
    where
        P: IntoIterator<Item = &'c [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        match drive_version.grove_methods.batch.batch_insert_empty_tree {
            0 => self.batch_insert_empty_tree_v0(path, key_info, storage_flags, drive_operations),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "batch_insert_empty_tree".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
