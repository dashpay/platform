use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::CalculatedCostOperation;
use grovedb::TransactionArg;
use grovedb_costs::CostContext;
use grovedb_path::SubtreePath;
use platform_version::version::drive_versions::DriveVersion;

impl Drive {
    /// Returns the total number of items in a CommitmentTree.
    /// Pushes the cost to `drive_operations` and returns the count.
    pub(super) fn grove_commitment_tree_count_v0<B: AsRef<[u8]>>(
        &self,
        path: SubtreePath<'_, B>,
        key: &[u8],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<u64, Error> {
        let CostContext { value, cost } = self.grove.commitment_tree_count(
            path,
            key,
            transaction,
            &drive_version.grove_version,
        );
        drive_operations.push(CalculatedCostOperation(cost));
        value.map_err(Error::from)
    }
}
