use crate::drive::{
    Drive
};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchInsertTreeApplyType;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;
use platform_version::version::PlatformVersion;
use crate::drive::tokens::token_balances_root_path;
use crate::util::object_size_info::PathKeyInfo::PathFixedSizeKey;

impl Drive {
    /// Creates a new token root subtree at `TokenBalances` keyed by `token_id`.
    /// This function applies the operations directly, calculates fees, and returns the fee result.
    pub(super) fn create_token_root_tree_v0(
        &self,
        token_id: [u8; 32],
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

        // Add operations to create the token root tree
        self.create_token_root_tree_add_to_operations_v0(
            token_id,
            apply,
            &mut None,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;

        // If applying, calculate fees
        let fees = Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            None,
        )?;

        Ok(fees)
    }

    /// Adds the token root creation operations to the provided `drive_operations` vector without
    /// calculating or returning fees. If `apply` is false, it will only estimate costs.
    pub(super) fn create_token_root_tree_add_to_operations_v0(
        &self,
        token_id: [u8; 32],
        apply: bool,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };

        // Get the operations required to create the token tree
        let batch_operations = self.create_token_root_tree_operations_v0(
            token_id,
            previous_batch_operations,
            &mut estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?;

        // Apply or estimate the operations
        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            drive_operations,
            &platform_version.drive,
        )
    }

    /// Gathers the operations needed to create the token root subtree. If `apply` is false, it
    /// populates `estimated_costs_only_with_layer_info` instead of applying.
    pub(super) fn create_token_root_tree_operations_v0(
        &self,
        token_id: [u8; 32],
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<HashMap<KeyInfoPath, EstimatedLayerInformation>>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut batch_operations: Vec<LowLevelDriveOperation> = vec![];

        // Decide if we're doing a stateful or stateless insert for cost estimation
        let apply_type = if estimated_costs_only_with_layer_info.is_none() {
            BatchInsertTreeApplyType::StatefulBatchInsertTree
        } else {
            BatchInsertTreeApplyType::StatelessBatchInsertTree {
                in_tree_using_sums: false,
                is_sum_tree: true,
                flags_len: 0, // No additional storage flags here
            }
        };

        // Insert an empty tree for this token if it doesn't exist
        let inserted = self.batch_insert_empty_tree_if_not_exists(
            PathFixedSizeKey((
                token_balances_root_path(),
                token_id.to_vec(),
            )),
            true,
            None, // No storage flags
            apply_type,
            transaction,
            previous_batch_operations,
            &mut batch_operations,
            &platform_version.drive,
        )?;

        if !inserted {
            // The token root already exists. Depending on your logic, this might be allowed or should be treated as an error.
            return Err(Error::Drive(DriveError::CorruptedDriveState(
                "token root tree already exists".to_string(),
            )));
        }

        Ok(batch_operations)
    }
}