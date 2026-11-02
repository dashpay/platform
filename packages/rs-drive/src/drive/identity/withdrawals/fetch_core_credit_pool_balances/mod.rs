mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::fee::Credits;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;
use std::ops::RangeInclusive;

impl Drive {
    /// Fetches the recorded Core credit pool balances (in credits) of the Core heights in
    /// `core_heights`. A height that was never recorded, or was pruned, is absent from the
    /// result.
    ///
    /// # Parameters
    ///
    /// * `core_heights`: The Core heights to read.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(BTreeMap<u32, Credits>)`: The balance of every recorded height in the range.
    /// * `Err(Error)` when the method version is unknown or not active, an entry is corrupted,
    ///   or the read fails.
    pub fn fetch_core_credit_pool_balances(
        &self,
        core_heights: RangeInclusive<u32>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<u32, Credits>, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .withdrawals
            .fetch_core_credit_pool_balances
        {
            Some(0) => {
                self.fetch_core_credit_pool_balances_v0(core_heights, transaction, platform_version)
            }
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_core_credit_pool_balances".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "fetch_core_credit_pool_balances".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
