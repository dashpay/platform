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
    /// Takes `amount` out of what is left of a key's budget and applies it, the way the fee of a
    /// state transition is applied to the balance of the identity that pays it.
    ///
    /// The remaining budget stops at zero: metered processing is allowed to take a key slightly
    /// over its budget, and a key at zero can no longer sign.
    ///
    /// # Parameters
    /// - `identity_id`: The identity the key belongs to.
    /// - `key_id`: The id of the budgeted key that signed the state transition.
    /// - `amount`: The credits the state transition took from the identity.
    /// - `transaction`: The transaction to apply in.
    /// - `platform_version`: The platform version selecting the implementation.
    ///
    /// # Returns
    /// - The budget that remains after the deduction. An error if the key has no budget entry.
    pub fn deduct_from_identity_key_budget(
        &self,
        identity_id: [u8; 32],
        key_id: KeyID,
        amount: Credits,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, Error> {
        let (batch_operations, remaining_budget) = self
            .deduct_from_identity_key_budget_operations(
                identity_id,
                key_id,
                amount,
                transaction,
                platform_version,
            )?;

        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.apply_batch_low_level_drive_operations(
            None,
            transaction,
            batch_operations,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        Ok(remaining_budget)
    }

    /// The operations that take `amount` out of what is left of a key's budget, and the budget
    /// that remains once they are applied.
    ///
    /// # Parameters
    /// - `identity_id`: The identity the key belongs to.
    /// - `key_id`: The id of the budgeted key.
    /// - `amount`: The credits to deduct.
    /// - `transaction`: The transaction the current remaining budget is read in.
    /// - `platform_version`: The platform version selecting the implementation.
    ///
    /// # Returns
    /// - The operations and the remaining budget. An error if the key has no budget entry.
    pub fn deduct_from_identity_key_budget_operations(
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
            .deduct_from_identity_key_budget
        {
            Some(0) => self.deduct_from_identity_key_budget_operations_v0(
                identity_id,
                key_id,
                amount,
                transaction,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "deduct_from_identity_key_budget_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "deduct_from_identity_key_budget_operations".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
