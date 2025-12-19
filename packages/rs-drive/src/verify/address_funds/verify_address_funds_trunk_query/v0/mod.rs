use crate::drive::Drive;
use crate::error::Error;
use crate::verify::RootHash;
use grovedb::{GroveDb, GroveTrunkQueryResult, PathTrunkChunkQuery};
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn verify_address_funds_trunk_query_v0(
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, GroveTrunkQueryResult), Error> {
        let path = Self::clear_addresses_path();
        let max_depth = platform_version
            .drive
            .methods
            .address_funds
            .address_funds_query_max_depth;

        let query = PathTrunkChunkQuery { path, max_depth };

        let (root_hash, result) = GroveDb::verify_trunk_chunk_proof(
            proof,
            &query,
            &platform_version.drive.grove_version,
        )?;

        Ok((root_hash, result))
    }
}
