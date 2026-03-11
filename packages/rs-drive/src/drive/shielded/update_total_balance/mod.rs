mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::PlatformVersion;

impl Drive {
    /// Constructs the low-level drive operations to update the shielded pool's
    /// total balance.
    ///
    /// # Parameters
    /// - `new_total_balance`: The new total balance value
    /// - `platform_version`: The platform version for dispatch
    pub fn update_total_balance_op(
        new_total_balance: u64,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version.drive.methods.shielded.update_total_balance {
            0 => Self::update_total_balance_op_v0(new_total_balance),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "update_total_balance_op".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
