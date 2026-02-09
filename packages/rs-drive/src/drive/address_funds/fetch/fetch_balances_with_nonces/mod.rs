mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::AddressNonce;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

impl Drive {
    /// Fetches the balances and nonces for multiple addresses from the AddressBalances tree.
    /// This operation retrieves the stored balances and nonces for all provided addresses.
    ///
    /// # Parameters
    /// - `addresses`: An iterator over platform addresses to look up
    /// - `transaction`: The transaction argument for the operation.
    /// - `platform_version`: The platform version to select the correct function version to run.
    ///
    /// # Returns
    /// - `Ok(BTreeMap<PlatformAddress, Option<(AddressNonce, Credits)>>)` - A map from addresses to optional (nonce, balance) pairs.
    ///   All input addresses are included in the result. Addresses that exist have `Some((nonce, balance))`,
    ///   addresses that don't exist have `None`.
    /// - `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    /// - `Err(Error)` if any other error occurs during the operation.
    pub fn fetch_balances_with_nonces<'a, I>(
        &self,
        addresses: I,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<PlatformAddress, Option<(AddressNonce, Credits)>>, Error>
    where
        I: IntoIterator<Item = &'a PlatformAddress>,
    {
        match platform_version
            .drive
            .methods
            .address_funds
            .fetch_balances_with_nonces
        {
            0 => self.fetch_balances_with_nonces_v0(addresses, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_balances_with_nonces".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
