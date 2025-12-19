use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use grovedb::{GroveDb, GroveBranchQueryResult, PathBranchChunkQuery};
use grovedb_merk::CryptoHash;
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn verify_address_funds_branch_query_v0(
        proof: &[u8],
        key: Vec<u8>,
        depth: u8,
        expected_root_hash: CryptoHash,
        platform_version: &PlatformVersion,
    ) -> Result<GroveBranchQueryResult, Error> {
        let min_depth = platform_version
            .drive
            .methods
            .address_funds
            .address_funds_query_min_depth;
        let max_depth = platform_version
            .drive
            .methods
            .address_funds
            .address_funds_query_max_depth;

        if depth < min_depth || depth > max_depth {
            return Err(Error::Drive(DriveError::InvalidInput(format!(
                "depth {} is outside the allowed range [{}, {}]",
                depth, min_depth, max_depth
            ))));
        }

        let path = Self::clear_addresses_path();
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
