use crate::drive::shielded::paths::nullifiers_path_for_pool;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::GroveDBToUse;
use dpp::version::PlatformVersion;
use grovedb::PathTrunkChunkQuery;

impl Drive {
    pub(super) fn prove_nullifiers_trunk_query_v0(
        &self,
        pool_type: u32,
        pool_identifier: Option<Vec<u8>>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        self.prove_nullifiers_trunk_query_operations_v0(
            pool_type,
            pool_identifier,
            &mut vec![],
            platform_version,
        )
    }

    pub(super) fn prove_nullifiers_trunk_query_operations_v0(
        &self,
        pool_type: u32,
        pool_identifier: Option<Vec<u8>>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let path = nullifiers_path_for_pool(pool_type, pool_identifier.as_deref())?;
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

        let query = PathTrunkChunkQuery {
            path,
            min_depth: Some(min_depth),
            max_depth,
        };

        self.grove_get_proved_trunk_chunk_query(
            &query,
            GroveDBToUse::LatestCheckpoint,
            drive_operations,
            &platform_version.drive,
        )
    }
}
