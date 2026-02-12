use crate::drive::shielded::paths::{
    shielded_credit_pool_nullifiers_path_vec, shielded_credit_pool_path_vec,
    SHIELDED_COMMITMENTS_KEY, SHIELDED_ENCRYPTED_NOTES_KEY, SHIELDED_TOTAL_BALANCE_KEY,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::GroveOperation;
use crate::util::batch::drive_op_batch::DriveLowLevelOperationConverter;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use grovedb::batch::{KeyInfoPath, QualifiedGroveDbOp};
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

/// Operations on the Shielded Pool
#[derive(Clone, Debug)]
pub enum ShieldedPoolOperationType {
    /// Append a note commitment (cmx) to the commitment tree
    AppendNoteCommitment {
        /// The 32-byte note commitment
        cmx: [u8; 32],
    },
    /// Insert a nullifier to prevent double-spend
    InsertNullifier {
        /// The 32-byte nullifier
        nullifier: [u8; 32],
    },
    /// Update the shielded pool total balance
    UpdateTotalBalance {
        /// The new total balance value
        new_total_balance: u64,
    },
    /// Insert encrypted notes with auto-incremented keys in the count tree
    InsertEncryptedNotes {
        /// Items to insert: each is cmx (32 bytes) || encrypted_note, packed as Element
        items: Vec<Element>,
    },
}

impl DriveLowLevelOperationConverter for ShieldedPoolOperationType {
    fn into_low_level_drive_operations(
        self,
        drive: &Drive,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        _block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        if let Some(ref mut estimated_costs) = estimated_costs_only_with_layer_info {
            Drive::add_estimation_costs_for_shielded_pool_operations(estimated_costs);
        }

        match self {
            ShieldedPoolOperationType::AppendNoteCommitment { cmx } => {
                let pool_path = shielded_credit_pool_path_vec();
                Ok(vec![GroveOperation(
                    QualifiedGroveDbOp::commitment_tree_append_op(
                        pool_path,
                        vec![SHIELDED_COMMITMENTS_KEY],
                        cmx,
                    ),
                )])
            }
            ShieldedPoolOperationType::InsertNullifier { nullifier } => {
                let nullifiers_path = shielded_credit_pool_nullifiers_path_vec();
                Ok(vec![GroveOperation(QualifiedGroveDbOp::insert_only_op(
                    nullifiers_path,
                    nullifier.to_vec(),
                    Element::new_item(vec![]),
                ))])
            }
            ShieldedPoolOperationType::UpdateTotalBalance { new_total_balance } => {
                let pool_path = shielded_credit_pool_path_vec();
                let balance_i64 = i64::try_from(new_total_balance).map_err(|_| {
                    Error::Drive(DriveError::CorruptedDriveState(
                        "shielded pool total balance exceeds i64::MAX".to_string(),
                    ))
                })?;
                Ok(vec![GroveOperation(
                    QualifiedGroveDbOp::insert_or_replace_op(
                        pool_path,
                        vec![SHIELDED_TOTAL_BALANCE_KEY],
                        Element::new_sum_item(balance_i64),
                    ),
                )])
            }
            ShieldedPoolOperationType::InsertEncryptedNotes { items } => {
                let mut ops = vec![];
                drive.batch_insert_auto_incremented_items_in_count_tree(
                    shielded_credit_pool_path_vec(),
                    &[SHIELDED_ENCRYPTED_NOTES_KEY],
                    items,
                    transaction,
                    &mut ops,
                    &platform_version.drive,
                )?;
                Ok(ops)
            }
        }
    }
}
