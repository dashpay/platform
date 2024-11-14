mod v0;

use crate::drive::identity::update::add_to_previous_balance_outcome::AddToPreviousBalanceOutcome;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::fee::Credits;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// The method to add balance to the previous balance. This function is version controlled.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The ID of the Identity.
    /// * `previous_balance` - The previous balance of the Identity.
    /// * `added_balance` - The balance to be added.
    /// * `apply` - Whether to apply the operations.
    /// * `transaction` - The current transaction.
    /// * `drive_operations` - The vector of LowLevelDriveOperations.
    /// * `drive_version` - The drive version.
    ///
    /// # Returns
    ///
    /// * `Result<AddToPreviousBalanceOutcome, Error>` - The outcome if successful, or an error.
    pub(in crate::drive::identity::update) fn add_to_previous_balance(
        &self,
        identity_id: [u8; 32],
        previous_balance: Credits,
        added_balance: Credits,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<AddToPreviousBalanceOutcome, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .update
            .add_to_previous_balance
        {
            0 => self.add_to_previous_balance_v0(
                identity_id,
                previous_balance,
                added_balance,
                apply,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_to_previous_balance".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
