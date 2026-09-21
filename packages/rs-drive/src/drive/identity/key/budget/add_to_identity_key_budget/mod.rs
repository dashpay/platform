mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::fee::Credits;
use dpp::identity::KeyID;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// The operations that add `amount` to what is left of a key's budget, and the budget that
    /// remains once they are applied. Used when the key's total budget is raised by the same
    /// amount, so `remaining <= total_budget` keeps holding and the addition cannot overflow.
    ///
    /// # Parameters
    /// - `identity_id`: The identity the key belongs to.
    /// - `key_id`: The id of the budgeted key.
    /// - `amount`: The credits to add.
    /// - `transaction`: The transaction the current remaining budget is read in.
    /// - `platform_version`: The platform version selecting the implementation.
    ///
    /// # Returns
    /// - The operations and the remaining budget. An error if the key has no budget entry.
    pub fn add_to_identity_key_budget_operations(
        &self,
        identity_id: [u8; 32],
        key_id: KeyID,
        amount: Credits,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Vec<LowLevelDriveOperation>, Credits), Error> {
        match platform_version
            .drive
            .methods
            .identity
            .keys
            .budget
            .add_to_identity_key_budget
        {
            Some(0) => self.add_to_identity_key_budget_operations_v0(
                identity_id,
                key_id,
                amount,
                transaction,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_to_identity_key_budget_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "add_to_identity_key_budget_operations".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
