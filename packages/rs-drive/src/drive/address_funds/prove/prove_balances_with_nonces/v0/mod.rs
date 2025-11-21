use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::identity::KeyOfType;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    pub(super) fn prove_balances_with_nonces_v0<'a, I>(
        &self,
        keys_of_type: I,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error>
    where
        I: IntoIterator<Item = &'a KeyOfType>,
    {
        self.prove_balances_with_nonces_operations_v0(
            keys_of_type,
            transaction,
            &mut vec![],
            platform_version,
        )
    }

    pub(super) fn prove_balances_with_nonces_operations_v0<'a, I>(
        &self,
        keys_of_type: I,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error>
    where
        I: IntoIterator<Item = &'a KeyOfType>,
    {
        let path_query = Drive::balances_for_addresses_query(keys_of_type);
        self.grove_get_proved_path_query(
            &path_query,
            transaction,
            drive_operations,
            &platform_version.drive,
        )
    }
}
