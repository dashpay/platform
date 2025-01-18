use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn prove_token_total_supply_and_aggregated_identity_balances_v0(
        &self,
        token_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        self.prove_token_total_supply_and_aggregated_identity_balances_add_operations_v0(
            token_id,
            transaction,
            &mut vec![],
            platform_version,
        )
    }

    pub(super) fn prove_token_total_supply_and_aggregated_identity_balances_add_operations_v0(
        &self,
        token_id: [u8; 32],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let combined_path_query = Drive::token_total_supply_and_aggregated_identity_balances_query(
            token_id,
            platform_version,
        )?;
        self.grove_get_proved_path_query(
            &combined_path_query,
            transaction,
            drive_operations,
            &platform_version.drive,
        )
    }
}
