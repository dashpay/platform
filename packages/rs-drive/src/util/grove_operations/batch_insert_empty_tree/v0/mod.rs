use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::object_size_info::DriveKeyInfo;
use crate::util::object_size_info::DriveKeyInfo::{Key, KeyRef, KeySize};
use crate::util::storage_flags::StorageFlags;
use grovedb::batch::KeyInfoPath;

impl Drive {
    /// Pushes an "insert empty tree" operation to `drive_operations`.
    pub(crate) fn batch_insert_empty_tree_v0<'a, 'c, P>(
        &'a self,
        path: P,
        key_info: DriveKeyInfo<'c>,
        storage_flags: Option<&StorageFlags>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<(), Error>
    where
        P: IntoIterator<Item = &'c [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        match key_info {
            KeyRef(key) => {
                let path_items: Vec<Vec<u8>> = path.into_iter().map(Vec::from).collect();
                drive_operations.push(LowLevelDriveOperation::for_known_path_key_empty_tree(
                    path_items,
                    key.to_vec(),
                    storage_flags,
                ));
                Ok(())
            }
            KeySize(key) => {
                drive_operations.push(LowLevelDriveOperation::for_estimated_path_key_empty_tree(
                    KeyInfoPath::from_known_path(path),
                    key,
                    storage_flags,
                ));
                Ok(())
            }
            Key(key) => {
                let path_items: Vec<Vec<u8>> = path.into_iter().map(Vec::from).collect();
                drive_operations.push(LowLevelDriveOperation::for_known_path_key_empty_tree(
                    path_items,
                    key,
                    storage_flags,
                ));
                Ok(())
            }
        }
    }
}
