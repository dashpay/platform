use crate::drive::document::ranked_index_tree_type::ranked_axes_tlv;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::object_size_info::DriveKeyInfo;
use crate::util::object_size_info::DriveKeyInfo::{Key, KeyRef, KeySize};
use crate::util::storage_flags::StorageFlags;
use grovedb::batch::KeyInfoPath;
use grovedb::element::IndexAxis;

impl Drive {
    /// Pushes an "insert empty provable count + provable sum indexed tree"
    /// (PCPSIT) operation to `drive_operations`. See module docs.
    pub(super) fn batch_insert_empty_provable_count_provable_sum_indexed_tree_v0<'a, 'c, P>(
        &'a self,
        path: P,
        key_info: DriveKeyInfo<'c>,
        ranked_axes: &[IndexAxis],
        storage_flags: Option<&StorageFlags>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<(), Error>
    where
        P: IntoIterator<Item = &'c [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        // Every secondary starts empty, so the TLV is `(tag, None)` per axis.
        // grovedb re-validates canonical ordering inside the constructor.
        let axes_tlv = ranked_axes_tlv(ranked_axes);
        match key_info {
            KeyRef(key) => {
                let path_items: Vec<Vec<u8>> = path.into_iter().map(Vec::from).collect();
                drive_operations.push(
                    LowLevelDriveOperation::for_known_path_key_empty_provable_count_provable_sum_indexed_tree(
                        path_items,
                        key.to_vec(),
                        axes_tlv,
                        storage_flags,
                    )?,
                );
                Ok(())
            }
            KeySize(key) => {
                drive_operations.push(
                    LowLevelDriveOperation::for_estimated_path_key_empty_provable_count_provable_sum_indexed_tree(
                        KeyInfoPath::from_known_path(path),
                        key,
                        axes_tlv,
                        storage_flags,
                    )?,
                );
                Ok(())
            }
            Key(key) => {
                let path_items: Vec<Vec<u8>> = path.into_iter().map(Vec::from).collect();
                drive_operations.push(
                    LowLevelDriveOperation::for_known_path_key_empty_provable_count_provable_sum_indexed_tree(
                        path_items,
                        key,
                        axes_tlv,
                        storage_flags,
                    )?,
                );
                Ok(())
            }
        }
    }
}
