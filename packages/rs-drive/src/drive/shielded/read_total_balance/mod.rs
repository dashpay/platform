mod v0;

use crate::drive::shielded::paths::token_shielded_pool_path;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::balances::credits::TokenAmount;
use dpp::fee::Credits;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Reads the current shielded pool total balance from GroveDB.
    ///
    /// Returns 0 if the balance key does not exist yet.
    ///
    /// # Parameters
    /// - `transaction`: The GroveDB transaction
    /// - `drive_operations`: A vector to collect the costs of operations
    /// - `platform_version`: The platform version for dispatch
    ///
    /// # Returns
    /// The current total balance of the shielded pool in credits.
    pub fn read_shielded_pool_total_balance(
        &self,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, Error> {
        match platform_version.drive.methods.shielded.read_total_balance {
            0 => self.read_shielded_pool_total_balance_v0(
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "read_shielded_pool_total_balance".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl Drive {
    /// Reads a TOKEN shielded pool's total balance (the amount of the token currently
    /// shielded). Returns 0 if the pool has no balance item yet. Same versioning as
    /// [`Drive::read_shielded_pool_total_balance`].
    pub fn read_token_shielded_pool_total_balance(
        &self,
        token_id: &[u8; 32],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<TokenAmount, Error> {
        match platform_version.drive.methods.shielded.read_total_balance {
            0 => {
                let pool_path = token_shielded_pool_path(token_id);
                self.read_pool_total_balance_v0(
                    &pool_path,
                    transaction,
                    drive_operations,
                    platform_version,
                )
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "read_token_shielded_pool_total_balance".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
