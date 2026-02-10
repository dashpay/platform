use crate::drive::shielded::paths::{
    shielded_credit_pool_encrypted_notes_path_vec, shielded_credit_pool_nullifiers_path_vec,
    shielded_credit_pool_path_vec, SHIELDED_COMMITMENTS_KEY,
    SHIELDED_TOTAL_BALANCE_KEY,
};
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::shielded::shield::ShieldTransitionAction;
use crate::state_transition_action::shielded::shielded_transfer::ShieldedTransferTransitionAction;
use crate::state_transition_action::shielded::unshield::UnshieldTransitionAction;
use crate::util::batch::drive_op_batch::finalize_task::DriveOperationFinalizeTask;
use crate::util::batch::drive_op_batch::AddressFundsOperationType;
use crate::util::batch::DriveOperation;
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;
use grovedb::batch::QualifiedGroveDbOp;
use grovedb::Element;

impl DriveHighLevelOperationConverter for ShieldTransitionAction {
    fn into_high_level_drive_operations<'a>(
        self,
        _epoch: &Epoch,
        _platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'a>>, Error> {
        match self {
            ShieldTransitionAction::V0(v0) => {
                let mut ops: Vec<DriveOperation<'a>> = Vec::new();

                // 1. Debit each input address: set remaining balance
                for (address, (nonce, remaining_balance)) in v0.inputs_with_remaining_balance {
                    ops.push(DriveOperation::AddressFundsOperation(
                        AddressFundsOperationType::SetBalanceToAddress {
                            address,
                            nonce,
                            balance: remaining_balance,
                        },
                    ));
                }

                let encrypted_notes_path = shielded_credit_pool_encrypted_notes_path_vec();
                let pool_path = shielded_credit_pool_path_vec();

                // 2. Append each note commitment to the commitment tree
                for cmx in v0.note_commitments.iter() {
                    ops.push(DriveOperation::ShieldedPoolOperation(
                        QualifiedGroveDbOp::commitment_tree_append_op(
                            pool_path.clone(),
                            vec![SHIELDED_COMMITMENTS_KEY],
                            *cmx,
                        ),
                    ));
                }

                // 3. Insert encrypted notes keyed by their commitment
                for (cmx, encrypted_note) in
                    v0.note_commitments.iter().zip(v0.encrypted_notes.iter())
                {
                    ops.push(DriveOperation::ShieldedPoolOperation(
                        QualifiedGroveDbOp::insert_only_op(
                            encrypted_notes_path.clone(),
                            cmx.to_vec(),
                            Element::new_item(encrypted_note.clone()),
                        ),
                    ));
                }

                // 4. Update total balance SumItem: current_total_balance + shield_amount
                let new_total_balance = v0.current_total_balance.checked_add(v0.shield_amount)
                    .ok_or_else(|| Error::Drive(
                        crate::error::drive::DriveError::CorruptedDriveState(
                            "shielded pool total balance overflow when adding shield amount".to_string(),
                        ),
                    ))?;
                ops.push(DriveOperation::ShieldedPoolOperation(
                    QualifiedGroveDbOp::insert_or_replace_op(
                        pool_path,
                        vec![SHIELDED_TOTAL_BALANCE_KEY],
                        Element::new_sum_item(new_total_balance as i64),
                    ),
                ));

                // Checkpoint the commitment tree and record anchor after batch is applied
                ops.push(DriveOperation::FinalizeOperation(
                    DriveOperationFinalizeTask::RecordShieldedAnchor,
                ));

                Ok(ops)
            }
        }
    }
}

impl DriveHighLevelOperationConverter for ShieldedTransferTransitionAction {
    fn into_high_level_drive_operations<'a>(
        self,
        _epoch: &Epoch,
        _platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'a>>, Error> {
        match self {
            ShieldedTransferTransitionAction::V0(v0) => {
                let mut ops: Vec<DriveOperation<'a>> = Vec::new();

                let nullifiers_path = shielded_credit_pool_nullifiers_path_vec();
                let encrypted_notes_path = shielded_credit_pool_encrypted_notes_path_vec();
                let pool_path = shielded_credit_pool_path_vec();

                // 1. Insert each nullifier (empty Item, InsertOnly to prevent double-spend)
                for nullifier in v0.nullifiers.iter() {
                    ops.push(DriveOperation::ShieldedPoolOperation(
                        QualifiedGroveDbOp::insert_only_op(
                            nullifiers_path.clone(),
                            nullifier.to_vec(),
                            Element::new_item(vec![]),
                        ),
                    ));
                }

                // 2. Append each note commitment to the commitment tree
                for cmx in v0.note_commitments.iter() {
                    ops.push(DriveOperation::ShieldedPoolOperation(
                        QualifiedGroveDbOp::commitment_tree_append_op(
                            pool_path.clone(),
                            vec![SHIELDED_COMMITMENTS_KEY],
                            *cmx,
                        ),
                    ));
                }

                // 3. Insert encrypted notes keyed by their commitment
                for (cmx, encrypted_note) in
                    v0.note_commitments.iter().zip(v0.encrypted_notes.iter())
                {
                    ops.push(DriveOperation::ShieldedPoolOperation(
                        QualifiedGroveDbOp::insert_only_op(
                            encrypted_notes_path.clone(),
                            cmx.to_vec(),
                            Element::new_item(encrypted_note.clone()),
                        ),
                    ));
                }

                // 4. Update total balance SumItem: subtract fee_amount
                let new_total_balance = v0
                    .current_total_balance
                    .checked_sub(v0.fee_amount)
                    .ok_or_else(|| Error::Drive(
                        crate::error::drive::DriveError::CorruptedDriveState(
                            "shielded pool total balance underflow when subtracting fee_amount".to_string(),
                        ),
                    ))?;
                ops.push(DriveOperation::ShieldedPoolOperation(
                    QualifiedGroveDbOp::insert_or_replace_op(
                        pool_path,
                        vec![SHIELDED_TOTAL_BALANCE_KEY],
                        Element::new_sum_item(new_total_balance as i64),
                    ),
                ));

                // Checkpoint the commitment tree and record anchor after batch is applied
                ops.push(DriveOperation::FinalizeOperation(
                    DriveOperationFinalizeTask::RecordShieldedAnchor,
                ));

                Ok(ops)
            }
        }
    }
}

impl DriveHighLevelOperationConverter for UnshieldTransitionAction {
    fn into_high_level_drive_operations<'a>(
        self,
        _epoch: &Epoch,
        _platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'a>>, Error> {
        match self {
            UnshieldTransitionAction::V0(v0) => {
                let mut ops: Vec<DriveOperation<'a>> = Vec::new();

                let nullifiers_path = shielded_credit_pool_nullifiers_path_vec();
                let encrypted_notes_path = shielded_credit_pool_encrypted_notes_path_vec();
                let pool_path = shielded_credit_pool_path_vec();

                // 1. Insert each nullifier (empty Item, InsertOnly to prevent double-spend)
                for nullifier in v0.nullifiers.iter() {
                    ops.push(DriveOperation::ShieldedPoolOperation(
                        QualifiedGroveDbOp::insert_only_op(
                            nullifiers_path.clone(),
                            nullifier.to_vec(),
                            Element::new_item(vec![]),
                        ),
                    ));
                }

                // 2. Credit the output address with the unshielded amount
                ops.push(DriveOperation::AddressFundsOperation(
                    AddressFundsOperationType::AddBalanceToAddress {
                        address: v0.output_address,
                        balance_to_add: v0.amount,
                    },
                ));

                // 3. Append each note commitment (change outputs) to the commitment tree
                for cmx in v0.note_commitments.iter() {
                    ops.push(DriveOperation::ShieldedPoolOperation(
                        QualifiedGroveDbOp::commitment_tree_append_op(
                            pool_path.clone(),
                            vec![SHIELDED_COMMITMENTS_KEY],
                            *cmx,
                        ),
                    ));
                }

                // 4. Insert encrypted notes keyed by their commitment
                for (cmx, encrypted_note) in
                    v0.note_commitments.iter().zip(v0.encrypted_notes.iter())
                {
                    ops.push(DriveOperation::ShieldedPoolOperation(
                        QualifiedGroveDbOp::insert_only_op(
                            encrypted_notes_path.clone(),
                            cmx.to_vec(),
                            Element::new_item(encrypted_note.clone()),
                        ),
                    ));
                }

                // 5. Update total balance SumItem: subtract the unshielded amount
                //    (fee is handled separately by the fee system)
                let new_total_balance = v0
                    .current_total_balance
                    .checked_sub(v0.amount)
                    .ok_or_else(|| Error::Drive(
                        crate::error::drive::DriveError::CorruptedDriveState(
                            "shielded pool total balance underflow when subtracting unshield amount".to_string(),
                        ),
                    ))?;
                ops.push(DriveOperation::ShieldedPoolOperation(
                    QualifiedGroveDbOp::insert_or_replace_op(
                        pool_path,
                        vec![SHIELDED_TOTAL_BALANCE_KEY],
                        Element::new_sum_item(new_total_balance as i64),
                    ),
                ));

                // Checkpoint the commitment tree and record anchor after batch is applied
                ops.push(DriveOperation::FinalizeOperation(
                    DriveOperationFinalizeTask::RecordShieldedAnchor,
                ));

                Ok(ops)
            }
        }
    }
}
