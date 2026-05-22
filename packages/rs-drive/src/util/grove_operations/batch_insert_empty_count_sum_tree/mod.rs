mod v0;

use crate::util::object_size_info::DriveKeyInfo;
use crate::util::storage_flags::StorageFlags;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::drive_versions::DriveVersion;

impl Drive {
    /// Pushes an "insert empty count-sum tree" operation to
    /// `drive_operations`. The `CountSumTree` variant carries a root-level
    /// `(count, sum)` pair without committing per-node aggregates, so
    /// O(1) total `count` and `sum` are available but range queries are
    /// not (those need `ProvableCountSumTree`).
    ///
    /// Used at contract creation when a document type opts into BOTH
    /// `documentsCountable` AND `documentsSummable` without any
    /// `range*` flags — i.e. callers want the doctype-level totals but
    /// not the per-node overhead of the provable variant. The
    /// dispatcher in `primary_key_tree_type.rs` v1 arm picks
    /// `TreeType::CountSumTree` for this combination.
    pub fn batch_insert_empty_count_sum_tree<'a, 'c, P>(
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
        match drive_version
            .grove_methods
            .batch
            .batch_insert_empty_count_sum_tree
        {
            0 => self.batch_insert_empty_count_sum_tree_v0(
                path,
                key_info,
                storage_flags,
                drive_operations,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "batch_insert_empty_count_sum_tree".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
