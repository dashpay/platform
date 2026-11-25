//! Version selection for pricing the balance work an erase causes.

mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::block::epoch::Epoch;
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;

impl Drive {
    /// Prices the worst-case third-party balance updates caused by one erase.
    ///
    /// # Parameters
    /// * `epoch`: The epoch whose fee schedule prices the recipient work.
    /// * `platform_version`: Selects the method and the erase chunk bound.
    ///
    /// # Returns
    /// The recipient-work fee without mutating balances, or an inactive/unknown version error.
    pub fn erase_refund_recipient_cost(
        &self,
        epoch: &Epoch,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match platform_version
            .drive
            .methods
            .document
            .delete
            .erase_refund_recipient_cost
        {
            Some(0) => self.erase_refund_recipient_cost_v0(epoch, platform_version),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "erase_refund_recipient_cost".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "erase_refund_recipient_cost".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
