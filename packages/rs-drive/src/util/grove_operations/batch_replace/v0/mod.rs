use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::object_size_info::PathKeyElementInfo;
use crate::util::object_size_info::PathKeyElementInfo::{
    PathFixedSizeKeyRefElement, PathKeyElement, PathKeyElementSize, PathKeyRefElement,
    PathKeyUnknownElementSize,
};

impl Drive {
    /// Pushes a "replace element" operation to `drive_operations`.
    pub(crate) fn batch_replace_v0<const N: usize>(
        &self,
        path_key_element_info: PathKeyElementInfo<N>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<(), Error> {
        match path_key_element_info {
            PathKeyRefElement((path, key, element)) => {
                drive_operations.push(LowLevelDriveOperation::replace_for_known_path_key_element(
                    path,
                    key.to_vec(),
                    element,
                ));
                Ok(())
            }
            PathKeyElement((path, key, element)) => {
                drive_operations.push(LowLevelDriveOperation::replace_for_known_path_key_element(
                    path, key, element,
                ));
                Ok(())
            }
            PathKeyElementSize((key_info_path, key_info, element)) => {
                drive_operations.push(
                    LowLevelDriveOperation::replace_for_estimated_path_key_element(
                        key_info_path,
                        key_info,
                        element,
                    ),
                );
                Ok(())
            }
            PathKeyUnknownElementSize(_) => Err(Error::Drive(DriveError::NotSupportedPrivate(
                "inserting unsized documents into a batch is not currently supported",
            ))),
            PathFixedSizeKeyRefElement((path, key, element)) => {
                let path_items: Vec<Vec<u8>> = path.into_iter().map(Vec::from).collect();
                drive_operations.push(LowLevelDriveOperation::replace_for_known_path_key_element(
                    path_items,
                    key.to_vec(),
                    element,
                ));
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::util::object_size_info::PathKeyElementInfo;
    use crate::util::test_helpers::setup::setup_drive;
    use grovedb::batch::key_info::KeyInfo;
    use grovedb::batch::KeyInfoPath;
    use grovedb::Element;

    #[test]
    fn test_batch_replace_path_key_ref_element() {
        let drive = setup_drive(None);
        let mut ops = vec![];
        let path = vec![b"root".to_vec()];
        let element = Element::new_item(b"value".to_vec());
        let info = PathKeyElementInfo::<0>::PathKeyRefElement((path, b"key", element));
        drive.batch_replace_v0(info, &mut ops).unwrap();
        assert_eq!(ops.len(), 1);
    }

    #[test]
    fn test_batch_replace_path_key_element() {
        let drive = setup_drive(None);
        let mut ops = vec![];
        let path = vec![b"root".to_vec()];
        let element = Element::new_item(b"value".to_vec());
        let info = PathKeyElementInfo::<0>::PathKeyElement((path, b"key".to_vec(), element));
        drive.batch_replace_v0(info, &mut ops).unwrap();
        assert_eq!(ops.len(), 1);
    }

    #[test]
    fn test_batch_replace_path_key_element_size() {
        let drive = setup_drive(None);
        let mut ops = vec![];
        let key_info_path = KeyInfoPath::from_known_owned_path(vec![b"root".to_vec()]);
        let key_info = KeyInfo::KnownKey(b"key".to_vec());
        let element = Element::new_item(b"value".to_vec());
        let info = PathKeyElementInfo::<0>::PathKeyElementSize((key_info_path, key_info, element));
        drive.batch_replace_v0(info, &mut ops).unwrap();
        assert_eq!(ops.len(), 1);
    }

    #[test]
    fn test_batch_replace_unknown_element_size_returns_error() {
        let drive = setup_drive(None);
        let mut ops = vec![];
        let key_info_path = KeyInfoPath::from_known_owned_path(vec![b"root".to_vec()]);
        let key_info = KeyInfo::KnownKey(b"key".to_vec());
        let info = PathKeyElementInfo::<0>::PathKeyUnknownElementSize((key_info_path, key_info, 8));
        let result = drive.batch_replace_v0(info, &mut ops);
        assert!(result.is_err());
    }

    #[test]
    fn test_batch_replace_fixed_size_key_ref_element() {
        let drive = setup_drive(None);
        let mut ops = vec![];
        let path: [&[u8]; 1] = [b"root"];
        let element = Element::new_item(b"value".to_vec());
        let info = PathKeyElementInfo::PathFixedSizeKeyRefElement((path, b"key", element));
        drive.batch_replace_v0(info, &mut ops).unwrap();
        assert_eq!(ops.len(), 1);
    }
}
