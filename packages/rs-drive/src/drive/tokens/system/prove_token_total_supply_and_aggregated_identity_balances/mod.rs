mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Proves token's total supply and aggregated identity balances
    pub fn prove_token_total_supply_and_aggregated_identity_balances(
        &self,
        token_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .token
            .prove
            .total_supply_and_aggregated_identity_balances
        {
            0 => self.prove_token_total_supply_and_aggregated_identity_balances_v0(
                token_id,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_token_total_supply_and_aggregated_identity_balances".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
