mod v0;

use crate::util::batch::DriveOperation;

use crate::drive::Drive;
use crate::error::{drive::DriveError, Error};

use dpp::block::block_info::BlockInfo;

use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::fee::fee_result::FeeResult;
use grovedb::TransactionArg;

use dpp::version::PlatformVersion;

impl Drive {
    /// Applies a list of high level DriveOperations to the drive, and calculates the fee for them.
    ///
    /// This method converts Drive operations to low-level operations, applies them if `apply` is true,
    /// and calculates the associated fee.
    ///
    /// # Arguments
    ///
    /// * `operations` - A vector of `DriveOperation`s to apply to the drive.
    /// * `apply` - A boolean flag indicating whether to apply the changes or only estimate costs.
    /// * `block_info` - A reference to information about the current block.
    /// * `transaction` - Transaction arguments.
    /// * `drive_version` - A `DriveVersion` reference that dictates which version of the method to call.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the `FeeResult` if the operations are successfully applied,
    /// otherwise an `Error`.
    ///
    pub fn apply_drive_operations(
        &self,
        operations: Vec<DriveOperation>,
        apply: bool,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
        previous_fee_versions: Option<&CachedEpochIndexFeeVersions>,
    ) -> Result<FeeResult, Error> {
        match platform_version
            .drive
            .methods
            .batch_operations
            .apply_drive_operations
        {
            0 => self.apply_drive_operations_v0(
                operations,
                apply,
                block_info,
                transaction,
                platform_version,
                previous_fee_versions,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "apply_drive_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
