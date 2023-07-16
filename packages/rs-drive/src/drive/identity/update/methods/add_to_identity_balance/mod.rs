mod v0;

use crate::drive::fee::calculate_fee;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::fee::Credits;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Balances are stored in the balance tree under the identity's id. This function is version controlled.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The ID of the Identity to which balance is to be added.
    /// * `added_balance` - The balance to be added.
    /// * `block_info` - The block information.
    /// * `apply` - Whether to apply the operations.
    /// * `transaction` - The current transaction.
    /// * `drive_version` - The drive version.
    ///
    /// # Returns
    ///
    /// * `Result<FeeResult, Error>` - The fee result if successful, or an error.
    pub fn add_to_identity_balance(
        &self,
        identity_id: [u8; 32],
        added_balance: Credits,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<FeeResult, Error> {
        match drive_version
            .methods
            .identity
            .update
            .add_to_identity_balance
        {
            0 => self.add_to_identity_balance_v0(
                identity_id,
                added_balance,
                block_info,
                apply,
                transaction,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_to_identity_balance".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
