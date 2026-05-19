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

#[cfg(test)]
mod tests {
    use crate::util::object_size_info::DriveKeyInfo;
    use crate::util::test_helpers::setup::setup_drive;
    use grovedb::batch::key_info::KeyInfo;

    #[test]
    fn test_batch_insert_empty_tree_key_ref() {
        let drive = setup_drive(None);
        let mut ops = vec![];
        let path: &[&[u8]] = &[b"root"];
        drive
            .batch_insert_empty_tree_v0(
                path.iter().copied(),
                DriveKeyInfo::KeyRef(b"child"),
                None,
                &mut ops,
            )
            .expect("expected operation to succeed");
        assert_eq!(ops.len(), 1);
    }

    #[test]
    fn test_batch_insert_empty_tree_key_size() {
        let drive = setup_drive(None);
        let mut ops = vec![];
        let path: &[&[u8]] = &[b"root"];
        drive
            .batch_insert_empty_tree_v0(
                path.iter().copied(),
                DriveKeyInfo::KeySize(KeyInfo::KnownKey(b"child".to_vec())),
                None,
                &mut ops,
            )
            .expect("expected operation to succeed");
        assert_eq!(ops.len(), 1);
    }

    #[test]
    fn test_batch_insert_empty_tree_key() {
        let drive = setup_drive(None);
        let mut ops = vec![];
        let path: &[&[u8]] = &[b"root"];
        drive
            .batch_insert_empty_tree_v0(
                path.iter().copied(),
                DriveKeyInfo::Key(b"child".to_vec()),
                None,
                &mut ops,
            )
            .expect("expected operation to succeed");
        assert_eq!(ops.len(), 1);
    }
}
