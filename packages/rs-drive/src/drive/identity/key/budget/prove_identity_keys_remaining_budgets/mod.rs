mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::identity::KeyID;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Proves what is left of the budgets of several keys of one identity. A key without a budget
    /// proves as absent. Verify with `Drive::verify_identity_keys_remaining_budgets`.
    ///
    /// # Parameters
    /// - `identity_id`: The identity the keys belong to.
    /// - `key_ids`: The ids of the keys to prove. Must not be empty.
    /// - `transaction`: The transaction to prove in.
    /// - `platform_version`: The platform version selecting the implementation.
    ///
    /// # Returns
    /// - The GroveDB proof.
    pub fn prove_identity_keys_remaining_budgets(
        &self,
        identity_id: [u8; 32],
        key_ids: &[KeyID],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .keys
            .budget
            .prove_identity_keys_remaining_budgets
        {
            Some(0) => self.prove_identity_keys_remaining_budgets_v0(
                identity_id,
                key_ids,
                transaction,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_identity_keys_remaining_budgets".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "prove_identity_keys_remaining_budgets".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
