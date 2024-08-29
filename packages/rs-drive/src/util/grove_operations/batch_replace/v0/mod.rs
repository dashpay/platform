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
