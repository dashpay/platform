mod v0;

use crate::util::object_size_info::DriveKeyInfo;
use crate::util::storage_flags::StorageFlags;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::drive_versions::DriveVersion;

impl Drive {
    /// Pushes an "insert empty provable count-sum tree" operation to
    /// `drive_operations`. The combined variant commits both per-node
    /// counts and per-node sums to every internal merk node — one tree
    /// carries both metrics, and a single range query can recover either
    /// (or both) without traversing leaves.
    ///
    /// Used at contract creation when an index declares BOTH
    /// `rangeCountable: true` AND `rangeSummable: true`, OR at the
    /// document-type primary-key level when both `rangeCountable` and
    /// `rangeSummable` are set. The dispatcher in
    /// `packages/rs-drive/src/drive/document/primary_key_tree_type.rs`'s
    /// v1 arm picks `TreeType::ProvableCountSumTree` for these cases.
    ///
    /// Lights up once grovedb PR 670 ships `Element::ProvableCountSumTree`
    /// as a callable empty-tree element.
    pub fn batch_insert_empty_provable_count_sum_tree<'a, 'c, P>(
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
            .batch_insert_empty_provable_count_sum_tree
        {
            0 => self.batch_insert_empty_provable_count_sum_tree_v0(
                path,
                key_info,
                storage_flags,
                drive_operations,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "batch_insert_empty_provable_count_sum_tree".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
