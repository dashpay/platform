use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::identity::KeyOfType;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    pub(super) fn prove_balance_and_nonce_v0(
        &self,
        key_of_type: &KeyOfType,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        self.prove_balance_and_nonce_operations_v0(
            key_of_type,
            transaction,
            &mut vec![],
            platform_version,
        )
    }

    pub(super) fn prove_balance_and_nonce_operations_v0(
        &self,
        key_of_type: &KeyOfType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let path_query = Drive::balance_for_address_query(key_of_type);
        self.grove_get_proved_path_query(
            &path_query,
            transaction,
            drive_operations,
            &platform_version.drive,
        )
    }
}
