mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::fee::Credits;
use dpp::identity::KeyID;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use std::collections::BTreeMap;

impl Drive {
    /// Fetches what is left of the budgets of several keys of one identity.
    ///
    /// # Parameters
    /// - `identity_id`: The identity the keys belong to.
    /// - `key_ids`: The ids of the keys to look up.
    /// - `transaction`: The transaction to read in.
    /// - `platform_version`: The platform version selecting the implementation.
    ///
    /// # Returns
    /// - One entry per requested key id: `Some(credits)` for a budgeted key, `None` for a key
    ///   without a budget, a key that does not exist, or an identity that does not exist.
    pub fn fetch_identity_keys_remaining_budgets(
        &self,
        identity_id: [u8; 32],
        key_ids: &[KeyID],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<KeyID, Option<Credits>>, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .keys
            .budget
            .fetch_identity_keys_remaining_budgets
        {
            Some(0) => self.fetch_identity_keys_remaining_budgets_v0(
                identity_id,
                key_ids,
                transaction,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_identity_keys_remaining_budgets".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "fetch_identity_keys_remaining_budgets".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
