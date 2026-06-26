use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::batch::drive_op_batch::DriveLowLevelOperationConverter;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

/// Operations on the Shielded Pool
#[derive(Clone, Debug)]
pub enum ShieldedPoolOperationType {
    /// Insert a note into the CommitmentTree (appends cmx to frontier + stores
    /// cmx||rho||cv_net||encrypted_note as item)
    InsertNote {
        /// The 32-byte nullifier (rho) of the spent note in this action, stored alongside
        /// the ciphertext so light clients can derive Rho for trial decryption
        nullifier: [u8; 32],
        /// The 32-byte note commitment (cmx)
        cmx: [u8; 32],
        /// The 32-byte value commitment (cv_net), stored unencrypted so a wallet can
        /// recover the value of an outgoing note via OVK decryption
        cv_net: [u8; 32],
        /// The encrypted note payload (216 bytes)
        encrypted_note: Vec<u8>,
    },
    /// Insert nullifiers into the permanent tree (double-spend prevention).
    InsertNullifiers {
        /// The 32-byte nullifiers to insert
        nullifiers: Vec<[u8; 32]>,
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
        drive: &Drive,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        _block_info: &BlockInfo,
        _transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        if let Some(ref mut estimated_costs) = estimated_costs_only_with_layer_info {
            Drive::add_estimation_costs_for_shielded_pool_operations(estimated_costs);
        }

        match self {
            ShieldedPoolOperationType::InsertNote {
                nullifier,
                cmx,
                cv_net,
                encrypted_note,
            } => Drive::insert_note_op(nullifier, cmx, cv_net, encrypted_note, platform_version),
            ShieldedPoolOperationType::InsertNullifiers { nullifiers } => {
                drive.insert_nullifiers(&nullifiers, platform_version)
            }
            ShieldedPoolOperationType::UpdateTotalBalance { new_total_balance } => {
                Drive::update_total_balance_op(new_total_balance, platform_version)
            }
        }
    }
}
