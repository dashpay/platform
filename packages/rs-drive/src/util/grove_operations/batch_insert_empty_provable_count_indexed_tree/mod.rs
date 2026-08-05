mod v0;

use crate::util::object_size_info::DriveKeyInfo;
use crate::util::storage_flags::StorageFlags;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::drive_versions::DriveVersion;

impl Drive {
    /// Pushes an "insert empty provable count-indexed tree" (PCIT, grovedb
    /// PR 657) operation to `drive_operations`.
    ///
    /// The indexed primary is a byte-compatible mirror of
    /// `ProvableCountTree` — every existing `AggregateCountOnRange` read
    /// keeps working against it unchanged. What the variant adds is one
    /// ordered secondary Merk keyed by `(count_be ‖ child_key)`, bound into
    /// the parent element alongside the primary root hash, so "top / bottom K
    /// groups by document count" resolves in O(log n + k) with a proof.
    ///
    /// Used at contract registration for an index that declares
    /// `rankedCountable: true` whose range layout is count-only (no
    /// `rangeSummable`). When the index also opts into the sum range axis its
    /// base layout is `ProvableCountProvableSumTree`, so it takes the
    /// multi-axis
    /// [`Self::batch_insert_empty_provable_count_provable_sum_indexed_tree`]
    /// path instead — even if Count is the only ranking axis declared.
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
    pub fn batch_insert_empty_provable_count_indexed_tree<'a, 'c, P>(
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
            .batch_insert_empty_provable_count_indexed_tree
        {
            0 => self.batch_insert_empty_provable_count_indexed_tree_v0(
                path,
                key_info,
                storage_flags,
                drive_operations,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "batch_insert_empty_provable_count_indexed_tree".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
