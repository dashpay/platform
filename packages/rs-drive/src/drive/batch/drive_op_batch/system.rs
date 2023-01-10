use crate::drive::batch::drive_op_batch::DriveOperationConverter;
use crate::drive::block_info::BlockInfo;
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::DriveOperation;
use dpp::identity::{Identity, IdentityPublicKey, KeyID, TimestampMillis};
use dpp::prelude::Revision;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;
use crate::fee::credits::Credits;

/// Operations on the System
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

impl DriveOperationConverter for SystemOperationType {
    fn to_drive_operations(
        self,
        drive: &Drive,
        _estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        _block_info: &BlockInfo,
        transaction: TransactionArg,
    ) -> Result<Vec<DriveOperation>, Error> {
        match self {
            SystemOperationType::AddToSystemCredits { amount } => drive
                .add_to_system_credits_operation(
                    amount,
                    transaction,
                ),
            SystemOperationType::RemoveFromSystemCredits { amount } => {
                drive.remove_from_system_credits_operations(amount, transaction)
            }
        }
    }
}
