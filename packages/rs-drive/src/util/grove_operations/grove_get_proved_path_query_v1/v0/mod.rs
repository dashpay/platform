use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::CalculatedCostOperation;
use grovedb::PathQuery;
use grovedb_costs::CostContext;
use platform_version::version::drive_versions::DriveVersion;

impl Drive {
    /// Gets the return value and the cost of a groveDB V1 proved path query.
    /// V1 proofs support BulkAppendTree and CommitmentTree subqueries.
    /// Pushes the cost to `drive_operations` and returns the proof bytes.
    pub(super) fn grove_get_proved_path_query_v1_v0(
        &self,
        path_query: &PathQuery,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<Vec<u8>, Error> {
        let CostContext { value, cost } =
            self.grove
                .prove_query(path_query, None, &drive_version.grove_version);
        drive_operations.push(CalculatedCostOperation(cost));
        value.map_err(Error::from)
    }
}
