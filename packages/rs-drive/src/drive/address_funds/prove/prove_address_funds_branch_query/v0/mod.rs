use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::GroveDBToUse;
use dpp::prelude::BlockHeight;
use dpp::version::PlatformVersion;
use grovedb::PathBranchChunkQuery;

impl Drive {
    pub(super) fn prove_address_funds_branch_query_v0(
        &self,
        key: Vec<u8>,
        depth: u8,
        checkpoint_height: BlockHeight,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        self.prove_address_funds_branch_query_operations_v0(
            key,
            depth,
            checkpoint_height,
            &mut vec![],
            platform_version,
        )
    }

    pub(super) fn prove_address_funds_branch_query_operations_v0(
        &self,
        key: Vec<u8>,
        depth: u8,
        checkpoint_height: BlockHeight,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
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

        self.grove_get_proved_branch_chunk_query(
            &query,
            GroveDBToUse::Checkpoint(checkpoint_height),
            drive_operations,
            &platform_version.drive,
        )
    }
}
