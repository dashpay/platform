use crate::drive::batch::drive_op_batch::DriveLowLevelOperationConverter;
use crate::drive::block_info::BlockInfo;
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::credits::Credits;
use crate::fee::op::LowLevelDriveOperation;

use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

/// Operations on the System
#[derive(Clone, Debug)]
pub enum SystemOperationType {
    /// We want to add credits to the system.
    AddToSystemCredits {
        /// The amount of credits we are seeking to add
        amount: Credits,
    },
    /// We want to remove credits from the system.
    RemoveFromSystemCredits {
        /// The amount of credits we are seeking to remove
        amount: Credits,
    },
}

impl DriveLowLevelOperationConverter for SystemOperationType {
    fn into_low_level_drive_operations(
        self,
        drive: &Drive,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        _block_info: &BlockInfo,
        transaction: TransactionArg,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match self {
            SystemOperationType::AddToSystemCredits { amount } => drive
                .add_to_system_credits_operation(
                    amount,
                    estimated_costs_only_with_layer_info,
                    transaction,
                ),
            SystemOperationType::RemoveFromSystemCredits { amount } => drive
                .remove_from_system_credits_operations(
                    amount,
                    estimated_costs_only_with_layer_info,
                    transaction,
                ),
        }
    }
}
