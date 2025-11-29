mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::fee::Credits;
use dpp::identity::KeyOfType;
use dpp::prelude::AddressNonce;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Fetches the balance and nonce for a given address from the AddressBalances tree.
    /// This operation retrieves the stored balance and nonce if they exist.
    ///
    /// # Parameters
    /// - `key_of_type`: The key (containing key type and key data) to look up
    /// - `transaction`: The transaction argument for the operation.
    /// - `platform_version`: The platform version to select the correct function version to run.
    ///
    /// # Returns
    /// - `Ok(Some((nonce, balance)))` if the address exists and has a balance
    /// - `Ok(None)` if the address does not exist
    /// - `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    /// - `Err(Error)` if any other error occurs during the operation.
    pub fn fetch_balance_and_nonce(
        &self,
        key_of_type: &KeyOfType,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<(AddressNonce, Credits)>, Error> {
        match platform_version
            .drive
            .methods
            .address_funds
            .fetch_balance_and_nonce
        {
            0 => self.fetch_balance_and_nonce_v0(key_of_type, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_balance_and_nonce".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
