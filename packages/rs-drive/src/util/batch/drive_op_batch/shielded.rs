use crate::drive::shielded::paths::{
    shielded_credit_pool_notes_path_vec, shielded_credit_pool_nullifiers_path_vec,
    shielded_credit_pool_path_vec, SHIELDED_TOTAL_BALANCE_KEY,
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
    /// Insert a note into the CommitmentTree (appends cmx to frontier + stores cmx||rho||encrypted_note as item)
    InsertNote {
        /// The 32-byte nullifier (rho) of the spent note in this action, stored alongside
        /// the ciphertext so light clients can derive Rho for trial decryption
        nullifier: [u8; 32],
        /// The 32-byte note commitment (cmx)
        cmx: [u8; 32],
        /// The encrypted note payload (216 bytes)
        encrypted_note: Vec<u8>,
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
    /// Store nullifiers to recent block storage for catch-up sync RPCs.
    /// Block height and time are taken from BlockInfo during low-level conversion.
    StoreNullifiersForBlock {
        /// The nullifiers to store for this block
        nullifiers: Vec<[u8; 32]>,
    },
}

impl DriveLowLevelOperationConverter for ShieldedPoolOperationType {
    fn into_low_level_drive_operations(
        self,
        drive: &Drive,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        if let Some(ref mut estimated_costs) = estimated_costs_only_with_layer_info {
            Drive::add_estimation_costs_for_shielded_pool_operations(estimated_costs);
        }

        match self {
            ShieldedPoolOperationType::InsertNote {
                nullifier,
                cmx,
                encrypted_note,
            } => {
                let notes_path = shielded_credit_pool_notes_path_vec();
                Ok(vec![GroveOperation(
                    QualifiedGroveDbOp::commitment_tree_insert_op(
                        notes_path,
                        cmx,
                        nullifier,
                        encrypted_note,
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
            ShieldedPoolOperationType::StoreNullifiersForBlock { nullifiers } => {
                // Store nullifiers to recent block storage for catch-up sync RPCs.
                // This is a side-effect operation — it doesn't produce low-level grove ops
                // but instead calls store_nullifiers_for_block directly.
                if !nullifiers.is_empty() {
                    drive.store_nullifiers_for_block(
                        &nullifiers,
                        block_info.height,
                        block_info.time_ms,
                        transaction,
                        platform_version,
                    )?;
                }
                Ok(vec![])
            }
        }
    }
}
