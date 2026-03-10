use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::push_drive_operation_result;
use grovedb::operations::insert::InsertOptions;
use grovedb::{Element, TransactionArg, TreeType};
use grovedb_path::SubtreePath;
use platform_version::version::drive_versions::DriveVersion;

impl Drive {
    /// Pushes the `OperationCost` of inserting an empty tree in groveDB to `drive_operations`.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn grove_insert_empty_tree_v0<B: AsRef<[u8]>>(
        &self,
        path: SubtreePath<'_, B>,
        key: &[u8],
        tree_type: TreeType,
        transaction: TransactionArg,
        options: Option<InsertOptions>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        let tree_element = match tree_type {
            TreeType::NormalTree => Element::empty_tree(),
            TreeType::SumTree => Element::empty_sum_tree(),
            TreeType::BigSumTree => Element::empty_big_sum_tree(),
            TreeType::CountTree => Element::empty_count_tree(),
            TreeType::CountSumTree => Element::empty_count_sum_tree(),
            TreeType::ProvableCountTree => Element::empty_provable_count_tree(),
            TreeType::ProvableCountSumTree => Element::empty_provable_count_sum_tree(),
            TreeType::CommitmentTree(chunk_power) => Element::empty_commitment_tree(chunk_power)?,
            TreeType::MmrTree => Element::empty_mmr_tree(),
            TreeType::BulkAppendTree(chunk_power) => Element::empty_bulk_append_tree(chunk_power)?,
            TreeType::DenseAppendOnlyFixedSizeTree(chunk_power) => {
                Element::empty_dense_tree(chunk_power)
            }
        };
        let cost_context = self.grove.insert(
            path,
            key,
            tree_element,
            options,
            transaction,
            &drive_version.grove_version,
        );
        push_drive_operation_result(cost_context, drive_operations)
    }
}
