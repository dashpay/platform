mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::block::block_info::BlockInfo;
use dpp::fee::Credits;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Records `amount` in the credit inflows sum tree under the withdrawals tree, keyed by
    /// the moment it expires, 25 hours after the block time. The daily withdrawal limit adds
    /// unexpired inflows younger than its day-old base snapshot to the daily maximum, making
    /// it a limit on net outflow. Called once per block with the block's total credit mints
    /// (asset locks, epoch Core rewards); an `amount` of zero records nothing.
    pub fn record_credit_inflow(
        &self,
        amount: Credits,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .identity
            .withdrawals
            .record_credit_inflows
        {
            Some(0) => {
                self.record_credit_inflow_v0(amount, block_info, transaction, platform_version)
            }
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "record_credit_inflow".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "record_credit_inflow".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
