mod v0;

use crate::drive::batch::{DriveOperation, GroveDbOpBatch};
use crate::drive::Drive;
use crate::error::{Error, DriveError};
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
use dpp::state_transition::fee::calculate_fee;
use dpp::state_transition::fee::fee_result::FeeResult;
use dpp::version::drive_versions::DriveVersion;
use crate::drive::batch::drive_op_batch::DriveLowLevelOperationConverter;

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
        drive_version: &DriveVersion,
    ) -> Result<FeeResult, Error> {
        match drive_version.methods.batch_operations.apply_drive_operations {
            0 => self.apply_drive_operations_v0(operations, apply, block_info, transaction, drive_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "apply_drive_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}