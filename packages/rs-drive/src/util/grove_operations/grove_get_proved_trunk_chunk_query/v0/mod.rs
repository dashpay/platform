use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::CalculatedCostOperation;
use crate::util::grove_operations::GroveDBToUse;
use grovedb::PathTrunkChunkQuery;
use grovedb_costs::CostContext;
use platform_version::version::drive_versions::DriveVersion;

impl Drive {
    /// Gets the return value and the cost of a groveDB proved trunk chunk query.
    /// Pushes the cost to `drive_operations` and returns the serialized proof.
    ///
    /// # Parameters
    /// * `query`: The trunk chunk query to retrieve a proof for.
    /// * `grove_db_to_use`: Which GroveDB instance to use (current, latest checkpoint, or specific checkpoint).
    /// * `drive_operations`: A vector to collect the costs of operations for later computation.
    /// * `drive_version`: The drive version to select the correct function version to run.
    pub(super) fn grove_get_proved_trunk_chunk_query_v0(
        &self,
        query: &PathTrunkChunkQuery,
        grove_db_to_use: GroveDBToUse,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<Vec<u8>, Error> {
        match grove_db_to_use {
            GroveDBToUse::Current => {
                let CostContext { value, cost } = self
                    .grove
                    .prove_trunk_chunk(query, &drive_version.grove_version);
                drive_operations.push(CalculatedCostOperation(cost));
                value.map_err(Error::from)
            }
            GroveDBToUse::LatestCheckpoint => {
                let checkpoints = self.checkpoints.load();
                let (_, (_, checkpoint)) = checkpoints
                    .last_key_value()
                    .ok_or(Error::Drive(DriveError::NoCheckpointsAvailable))?;
                let CostContext { value, cost } = checkpoint
                    .grove_db
                    .prove_trunk_chunk(query, &drive_version.grove_version);
                drive_operations.push(CalculatedCostOperation(cost));
                value.map_err(Error::from)
            }
            GroveDBToUse::Checkpoint(block_height) => {
                let checkpoints = self.checkpoints.load();
                let (_, checkpoint) = checkpoints
                    .get(&block_height)
                    .ok_or(Error::Drive(DriveError::CheckpointNotFound(block_height)))?;
                let CostContext { value, cost } = checkpoint
                    .grove_db
                    .prove_trunk_chunk(query, &drive_version.grove_version);
                drive_operations.push(CalculatedCostOperation(cost));
                value.map_err(Error::from)
            }
        }
    }
}
