mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::fee::Credits;
use dpp::withdrawal::WithdrawalTransactionIndex;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

/// What the pooled withdrawal transactions not completed yet take out of Core's credit pool
/// once mined, in credits.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InFlightWithdrawalAmounts {
    /// The sum over the queued transactions, which Core has not seen yet.
    pub queued: Credits,
    /// Each broadcast transaction's amount by its index: Core may have mined some of them
    /// already, which the broadcast tree only learns of a bounded number at a time.
    pub broadcast: BTreeMap<WithdrawalTransactionIndex, Credits>,
}

impl Drive {
    /// Reads what the pooled withdrawal transactions not completed yet (the queue and the
    /// broadcast tree) will take out of Core's credit pool once mined, in credits: each
    /// transaction's outputs plus its fee, as Core counts an asset unlock. Core's own unlock
    /// limit only reflects unlocks already mined, so the Core-anchored withdrawal limit
    /// subtracts the queued sum and the broadcast transactions Core has not mined.
    ///
    /// # Parameters
    ///
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(InFlightWithdrawalAmounts)`: The queued sum and the broadcast amounts, in credits.
    /// * `Err(Error)` when the method version is unknown or not active, a stored transaction
    ///   cannot be decoded, or the sum overflows.
    pub fn fetch_in_flight_withdrawal_amount(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<InFlightWithdrawalAmounts, Error> {
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
