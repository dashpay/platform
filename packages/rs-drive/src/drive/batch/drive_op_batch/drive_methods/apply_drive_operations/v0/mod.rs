use crate::drive::batch::DriveOperation;

use crate::drive::Drive;
use crate::error::Error;

use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;

use grovedb::{EstimatedLayerInformation, TransactionArg};

use crate::drive::batch::drive_op_batch::DriveLowLevelOperationConverter;

use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;

use crate::drive::batch::drive_op_batch::finalize_task::{
    DriveOperationFinalizationTasks, DriveOperationFinalizeTask,
};
use std::collections::HashMap;

impl Drive {
    /// Applies a list of high level DriveOperations to the drive, and calculates the fee for them.
    ///
    /// # Arguments
    ///
    /// * `operations` - A vector of `DriveOperation`s to apply to the drive.
    /// * `apply` - A boolean flag indicating whether to apply the changes or only estimate costs.
    /// * `block_info` - A reference to information about the current block.
    /// * `transaction` - Transaction arguments.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the `FeeResult` if the operations are successfully applied,
    /// otherwise an `Error`.
    ///
    /// If `apply` is set to true, it applies the low-level drive operations and updates side info accordingly.
    /// If not, it only estimates the costs and updates estimated costs with layer info.
    #[inline(always)]
    pub(super) fn apply_drive_operations_v0(
        &self,
        operations: Vec<DriveOperation>,
        apply: bool,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        if operations.is_empty() {
            return Ok(FeeResult::default());
        }
        let mut low_level_operations = vec![];
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };

        let mut finalize_tasks: Vec<DriveOperationFinalizeTask> = Vec::new();

        for drive_op in operations {
            if let Some(tasks) = drive_op.finalization_tasks(platform_version)? {
                finalize_tasks.extend(tasks);
            }

            low_level_operations.append(&mut drive_op.into_low_level_drive_operations(
                self,
                &mut estimated_costs_only_with_layer_info,
                block_info,
                transaction,
                platform_version,
            )?);
        }

        let mut cost_operations = vec![];

        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            low_level_operations,
            &mut cost_operations,
            &platform_version.drive,
        )?;

        // Execute drive operation callbacks after updating state
        for task in finalize_tasks {
            task.execute(self, platform_version);
        }

        Drive::calculate_fee(
            None,
            Some(cost_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
        )
    }
}
