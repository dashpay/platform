use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::CalculatedCostOperation;
use grovedb::PathTrunkChunkQuery;
use grovedb_costs::CostContext;
use platform_version::version::drive_versions::DriveVersion;

impl Drive {
    /// Gets the return value and the cost of a groveDB proved trunk chunk query.
    /// Pushes the cost to `drive_operations` and returns the serialized proof.
    pub(super) fn grove_get_proved_trunk_chunk_query_v0(
        &self,
        query: &PathTrunkChunkQuery,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<Vec<u8>, Error> {
        let CostContext { value, cost } = self
            .grove
            .prove_trunk_chunk(query, &drive_version.grove_version);
        drive_operations.push(CalculatedCostOperation(cost));
        value.map_err(Error::from)
    }
}
