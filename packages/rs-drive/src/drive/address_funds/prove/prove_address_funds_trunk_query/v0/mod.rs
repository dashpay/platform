use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::PlatformVersion;
use grovedb::PathTrunkChunkQuery;

impl Drive {
    pub(super) fn prove_address_funds_trunk_query_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        self.prove_address_funds_trunk_query_operations_v0(&mut vec![], platform_version)
    }

    pub(super) fn prove_address_funds_trunk_query_operations_v0(
        &self,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let path = Self::clear_addresses_path();
        let max_depth = platform_version
            .drive
            .methods
            .address_funds
            .address_funds_query_max_depth;

        let query = PathTrunkChunkQuery { path, max_depth };

        self.grove_get_proved_trunk_chunk_query(&query, drive_operations, &platform_version.drive)
    }
}
