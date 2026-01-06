use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::address_funds::PlatformAddress;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    pub(super) fn prove_balance_and_nonce_v0(
        &self,
        address: &PlatformAddress,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        self.prove_balance_and_nonce_operations_v0(
            address,
            transaction,
            &mut vec![],
            platform_version,
        )
    }

    pub(super) fn prove_balance_and_nonce_operations_v0(
        &self,
        address: &PlatformAddress,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let path_query = Drive::balance_for_clear_address_query(address);
        self.grove_get_proved_path_query(
            &path_query,
            transaction,
            drive_operations,
            &platform_version.drive,
        )
    }
}
