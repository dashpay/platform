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
    /// Fetches what is left of the budget of an identity's key.
    ///
    /// # Parameters
    /// - `identity_id`: The identity the key belongs to.
    /// - `key_id`: The id of the key.
    /// - `transaction`: The transaction to read in.
    /// - `platform_version`: The platform version selecting the implementation.
    ///
    /// # Returns
    /// - `Ok(Some(credits))` for a budgeted key, `Ok(None)` when the key has no budget entry
    ///   (it is not budgeted, or it does not exist).
    pub fn fetch_identity_key_remaining_budget(
        &self,
        identity_id: [u8; 32],
        key_id: KeyID,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Credits>, Error> {
        let mut drive_operations = vec![];
        self.fetch_identity_key_remaining_budget_operations(
            identity_id,
            key_id,
            transaction,
            &mut drive_operations,
            platform_version,
        )
    }

    /// Fetches what is left of the budget of an identity's key, adding the cost of the read to
    /// `drive_operations`. The read is always stateful: there is no budget to estimate.
    ///
    /// # Parameters
    /// - `identity_id`: The identity the key belongs to.
    /// - `key_id`: The id of the key.
    /// - `transaction`: The transaction to read in.
    /// - `drive_operations`: The operations the read is appended to.
    /// - `platform_version`: The platform version selecting the implementation.
    ///
    /// # Returns
    /// - `Ok(Some(credits))` for a budgeted key, `Ok(None)` when the key has no budget entry.
    pub(crate) fn fetch_identity_key_remaining_budget_operations(
        &self,
        identity_id: [u8; 32],
        key_id: KeyID,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Credits>, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .keys
            .budget
            .fetch_identity_key_remaining_budget
        {
            Some(0) => self.fetch_identity_key_remaining_budget_operations_v0(
                identity_id,
                key_id,
                transaction,
                drive_operations,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_identity_key_remaining_budget_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "fetch_identity_key_remaining_budget_operations".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
