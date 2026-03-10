use crate::drive::shielded::paths::nullifiers_path_for_pool;
use crate::drive::Drive;
use crate::error::Error;
use crate::verify::RootHash;
use grovedb::{GroveDb, GroveTrunkQueryResult, PathTrunkChunkQuery};
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn verify_nullifiers_trunk_query_v0(
        proof: &[u8],
        pool_type: u32,
        pool_identifier: Option<&[u8]>,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, GroveTrunkQueryResult), Error> {
        let path = nullifiers_path_for_pool(pool_type, pool_identifier)?;
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

        let (root_hash, result) = GroveDb::verify_trunk_chunk_proof(
            proof,
            &query,
            &platform_version.drive.grove_version,
        )?;

        Ok((root_hash, result))
    }
}
