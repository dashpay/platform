mod v0;

use crate::util::object_size_info::DriveKeyInfo;
use crate::util::storage_flags::StorageFlags;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::drive_versions::DriveVersion;
use grovedb::element::IndexAxis;

impl Drive {
    /// Pushes an "insert empty provable count + provable sum indexed tree"
    /// (PCPSIT, grovedb PR 657) operation to `drive_operations`.
    ///
    /// The multi-axis indexed variant: the primary is a byte-compatible
    /// mirror of `ProvableCountProvableSumTree` (so both
    /// `AggregateCountOnRange` and `AggregateSumOnRange` keep working
    /// unchanged), and the parent element carries a TLV list of 1..=3 ordered
    /// secondaries — one per declared ranking axis, keyed by that axis's
    /// order-preserving encoding of the child's aggregate.
    ///
    /// This is the arm every ranked index takes except the two pure
    /// single-axis shapes; in particular an index that ranks by Count alone
    /// but also declares `rangeSummable` lands here, because its primary must
    /// stay PCPS-shaped for the sum-on-range reads.
    ///
    /// # Parameters
    /// * `path`: The path to insert an empty tree.
    /// * `key_info`: The key information of the document.
    /// * `ranked_axes`: The declared ranking axes, canonically ordered
    ///   (Count < Sum < Avg) and non-empty. grovedb validates the resulting
    ///   TLV — an empty, unsorted or duplicated list is rejected.
    /// * `storage_flags`: Storage options for the operation.
    /// * `drive_operations`: The vector containing low-level drive operations.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(())` if the operation was successful.
    /// * `Err(Error::GroveDB)` if `ranked_axes` is not a canonical axis list.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    pub fn batch_insert_empty_provable_count_provable_sum_indexed_tree<'a, 'c, P>(
        &'a self,
        path: P,
        key_info: DriveKeyInfo<'c>,
        ranked_axes: &[IndexAxis],
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
            .batch_insert_empty_provable_count_provable_sum_indexed_tree
        {
            0 => self.batch_insert_empty_provable_count_provable_sum_indexed_tree_v0(
                path,
                key_info,
                ranked_axes,
                storage_flags,
                drive_operations,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "batch_insert_empty_provable_count_provable_sum_indexed_tree".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
