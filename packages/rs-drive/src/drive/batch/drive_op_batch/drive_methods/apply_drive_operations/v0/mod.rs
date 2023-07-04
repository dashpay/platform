use crate::drive::batch::{DriveOperation, GroveDbOpBatch};
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::calculate_fee;
use crate::fee::op::LowLevelDriveOperation;
use crate::fee::result::FeeResult;
use dpp::block::extended_block_info::BlockInfo;

pub use contract::ContractOperationType;
pub use document::DocumentOperation;
pub use document::DocumentOperationType;
pub use document::DocumentOperationsForContractDocumentType;
pub use document::UpdateOperationInfo;
pub use identity::IdentityOperationType;
pub use system::SystemOperationType;
pub use withdrawals::WithdrawalOperationType;

use grovedb::{EstimatedLayerInformation, TransactionArg};

use crate::fee::op::LowLevelDriveOperation::GroveOperation;
use grovedb::batch::{GroveDbOp, KeyInfoPath};
use itertools::Itertools;
use std::collections::{BTreeMap, HashMap};
use crate::fee::calculate_fee;
use dpp::state_transition::fee::fee_result::FeeResult;
use dpp::version::drive_versions::DriveVersion;
use crate::drive::batch::drive_op_batch::DriveLowLevelOperationConverter;

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
    pub(super) fn apply_drive_operations_v0(
        &self,
        operations: Vec<DriveOperation>,
        apply: bool,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
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
        for drive_op in operations {
            low_level_operations.append(&mut drive_op.into_low_level_drive_operations(
                self,
                &mut estimated_costs_only_with_layer_info,
                block_info,
                transaction,
                drive_version,
            )?);
        }
        let mut cost_operations = vec![];
        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            low_level_operations,
            &mut cost_operations,
            drive_version,
        )?;
        calculate_fee(None, Some(cost_operations), &block_info.epoch).map_err(Error::Protocol)
    }
}