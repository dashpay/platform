mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::block::epoch::Epoch;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::FeeMultiplier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Reads the fee multiplier, in permille, that document action fees priced by it are
    /// scaled with in `epoch`, and the fee of the read so that consensus validation can bill
    /// it.
    ///
    /// It is the multiplier recorded in the epoch's tree. In the first block of an epoch that
    /// item is not there yet: state transitions execute before the end of the block, where the
    /// epoch is initialized, so the multiplier the epoch is about to be initialized with is
    /// used, the one of the fee schedule. Whatever later sets an epoch's multiplier has to
    /// feed both.
    ///
    /// # Parameters
    ///
    /// * `epoch`: The epoch the action executes in.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok((FeeResult, FeeMultiplier))` with the fee of the read and the multiplier.
    /// * `Err(Error)` when the version is unknown, the read fails, or the item is malformed.
    pub fn fetch_action_fee_multiplier_with_fee(
        &self,
        epoch: &Epoch,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(FeeResult, FeeMultiplier), Error> {
        match platform_version
            .drive
            .methods
            .contract
            .fee_pots
            .fetch_action_fee_multiplier
        {
            0 => self.fetch_action_fee_multiplier_with_fee_v0(epoch, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_action_fee_multiplier_with_fee".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
