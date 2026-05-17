use crate::drive::shielded::paths::nullifiers_path_for_pool;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use grovedb::{GroveBranchQueryResult, GroveDb, PathBranchChunkQuery};
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn verify_nullifiers_branch_query_v0(
        proof: &[u8],
        pool_type: u32,
        pool_identifier: Option<&[u8]>,
        key: Vec<u8>,
        depth: u8,
        expected_root_hash: [u8; 32],
        platform_version: &PlatformVersion,
    ) -> Result<GroveBranchQueryResult, Error> {
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

        let path = nullifiers_path_for_pool(pool_type, pool_identifier)?;
        let query = PathBranchChunkQuery { path, key, depth };

        let result = GroveDb::verify_branch_chunk_proof(
            proof,
            &query,
            expected_root_hash,
            &platform_version.drive.grove_version,
        )?;

        Ok(result)
    }
}
