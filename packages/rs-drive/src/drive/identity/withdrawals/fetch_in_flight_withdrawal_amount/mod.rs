mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::fee::Credits;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Reads what the pooled withdrawal transactions not completed yet (the queue and the
    /// broadcast tree) will take out of Core's credit pool once mined, in credits: each
    /// transaction's outputs plus its fee, as Core counts an asset unlock. Core's own unlock
    /// limit only reflects unlocks already mined, so the Core-anchored withdrawal limit
    /// subtracts this sum.
    ///
    /// # Parameters
    ///
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(Credits)`: The sum over the queued and broadcast transactions, in credits.
    /// * `Err(Error)` when the method version is unknown or not active, a stored transaction
    ///   cannot be decoded, or the sum overflows.
    pub fn fetch_in_flight_withdrawal_amount(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .withdrawals
            .fetch_in_flight_withdrawal_amount
        {
            Some(0) => self.fetch_in_flight_withdrawal_amount_v0(transaction, platform_version),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_in_flight_withdrawal_amount".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "fetch_in_flight_withdrawal_amount".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
