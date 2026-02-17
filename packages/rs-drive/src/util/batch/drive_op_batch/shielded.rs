use crate::drive::shielded::paths::{
    shielded_credit_pool_nullifiers_path_vec, shielded_credit_pool_path_vec, SHIELDED_NOTES_KEY,
    SHIELDED_TOTAL_BALANCE_KEY,
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
    /// Insert a note into the CommitmentTree (appends cmx to frontier + stores nullifier||cmx||payload as item)
    InsertNote {
        /// The 32-byte nullifier of the spent note in this action (needed for Rho derivation in trial decryption)
        nullifier: [u8; 32],
        /// The 32-byte note commitment (cmx)
        cmx: [u8; 32],
        /// The encrypted note payload
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
}

impl DriveLowLevelOperationConverter for ShieldedPoolOperationType {
    fn into_low_level_drive_operations(
        self,
        _drive: &Drive,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        _block_info: &BlockInfo,
        _transaction: TransactionArg,
        _platform_version: &PlatformVersion,
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
                let pool_path = shielded_credit_pool_path_vec();
                // Payload = nullifier || encrypted_note
                // Retrieved value = cmx || nullifier || encrypted_note
                // The nullifier is needed by light clients to derive Rho for trial decryption
                let mut payload = Vec::with_capacity(32 + encrypted_note.len());
                payload.extend_from_slice(&nullifier);
                payload.extend_from_slice(&encrypted_note);
                Ok(vec![GroveOperation(
                    QualifiedGroveDbOp::commitment_tree_insert_op(
                        pool_path,
                        vec![SHIELDED_NOTES_KEY],
                        cmx,
                        payload,
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
        }
    }
}
