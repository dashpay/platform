use crate::drive::shielded::paths::nullifiers_path_for_pool;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::GroveDBToUse;
use dpp::prelude::BlockHeight;
use dpp::version::PlatformVersion;
use grovedb::PathBranchChunkQuery;

impl Drive {
    pub(super) fn prove_nullifiers_branch_query_v0(
        &self,
        pool_type: u32,
        pool_identifier: Option<Vec<u8>>,
        key: Vec<u8>,
        depth: u8,
        checkpoint_height: BlockHeight,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        self.prove_nullifiers_branch_query_operations_v0(
            pool_type,
            pool_identifier,
            key,
            depth,
            checkpoint_height,
            &mut vec![],
            platform_version,
        )
    }

    pub(super) fn prove_nullifiers_branch_query_operations_v0(
        &self,
        pool_type: u32,
        pool_identifier: Option<Vec<u8>>,
        key: Vec<u8>,
        depth: u8,
        checkpoint_height: BlockHeight,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let min_depth = platform_version
            .drive
            .methods
            .shielded
            .nullifiers_query_min_depth;
        let max_depth = platform_version
            .drive
            .methods
            .shielded
            .nullifiers_query_max_depth;

        if depth < min_depth || depth > max_depth {
            return Err(Error::Drive(DriveError::InvalidInput(format!(
                "depth {} is outside the allowed range [{}, {}]",
                depth, min_depth, max_depth
            ))));
        }

        let path = nullifiers_path_for_pool(pool_type, pool_identifier.as_deref())?;
        let query = PathBranchChunkQuery { path, key, depth };

        self.grove_get_proved_branch_chunk_query(
            &query,
            GroveDBToUse::Checkpoint(checkpoint_height),
            drive_operations,
            &platform_version.drive,
        )
    }
}
