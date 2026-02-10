use crate::drive::shielded::paths::{
    shielded_credit_pool_commitments_path_vec, shielded_credit_pool_encrypted_notes_path_vec,
    shielded_credit_pool_nullifiers_path_vec, shielded_credit_pool_path_vec, SHIELDED_PARAMS_KEY,
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
use dpp::shielded::ShieldedPoolParams;
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

                let commitments_path = shielded_credit_pool_commitments_path_vec();
                let encrypted_notes_path = shielded_credit_pool_encrypted_notes_path_vec();
                let pool_path = shielded_credit_pool_path_vec();

                // We start with the current checkpoint_id and increment for each commitment
                let mut next_checkpoint_id = v0.current_checkpoint_id;

                // 2. Append each note commitment to the commitment tree
                for cmx in v0.note_commitments.iter() {
                    next_checkpoint_id += 1;
                    ops.push(DriveOperation::GroveDBOperation(
                        QualifiedGroveDbOp::commitment_tree_append_op(
                            commitments_path.clone(),
                            vec![], // key is empty for commitment tree append
                            *cmx,
                            next_checkpoint_id,
                        ),
                    ));
                }

                // 3. Insert encrypted notes keyed by their commitment
                for (cmx, encrypted_note) in
                    v0.note_commitments.iter().zip(v0.encrypted_notes.iter())
                {
                    ops.push(DriveOperation::GroveDBOperation(
                        QualifiedGroveDbOp::insert_only_op(
                            encrypted_notes_path.clone(),
                            cmx.to_vec(),
                            Element::new_item(encrypted_note.clone()),
                        ),
                    ));
                }

                // 4. Update total balance SumItem: current_total_balance + shield_amount
                let new_total_balance = v0.current_total_balance + v0.shield_amount;
                ops.push(DriveOperation::GroveDBOperation(
                    QualifiedGroveDbOp::insert_or_replace_op(
                        pool_path.clone(),
                        vec![SHIELDED_TOTAL_BALANCE_KEY],
                        Element::new_sum_item(new_total_balance as i64),
                    ),
                ));

                // 5. Update params with incremented checkpoint_id_counter
                let new_params = ShieldedPoolParams {
                    checkpoint_id_counter: next_checkpoint_id,
                };
                let encoded_params =
                    bincode::encode_to_vec(&new_params, bincode::config::standard())
                        .expect("expected to encode shielded pool params");
                ops.push(DriveOperation::GroveDBOperation(
                    QualifiedGroveDbOp::insert_or_replace_op(
                        pool_path,
                        vec![SHIELDED_PARAMS_KEY],
                        Element::new_item(encoded_params),
                    ),
                ));

                // Record anchor after batch is applied (finalization task)
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
                let commitments_path = shielded_credit_pool_commitments_path_vec();
                let encrypted_notes_path = shielded_credit_pool_encrypted_notes_path_vec();
                let pool_path = shielded_credit_pool_path_vec();

                // 1. Insert each nullifier (empty Item, InsertOnly to prevent double-spend)
                for nullifier in v0.nullifiers.iter() {
                    ops.push(DriveOperation::GroveDBOperation(
                        QualifiedGroveDbOp::insert_only_op(
                            nullifiers_path.clone(),
                            nullifier.to_vec(),
                            Element::new_item(vec![]),
                        ),
                    ));
                }

                // We start with the current checkpoint_id and increment for each commitment
                let mut next_checkpoint_id = v0.current_checkpoint_id;

                // 2. Append each note commitment to the commitment tree
                for cmx in v0.note_commitments.iter() {
                    next_checkpoint_id += 1;
                    ops.push(DriveOperation::GroveDBOperation(
                        QualifiedGroveDbOp::commitment_tree_append_op(
                            commitments_path.clone(),
                            vec![],
                            *cmx,
                            next_checkpoint_id,
                        ),
                    ));
                }

                // 3. Insert encrypted notes keyed by their commitment
                for (cmx, encrypted_note) in
                    v0.note_commitments.iter().zip(v0.encrypted_notes.iter())
                {
                    ops.push(DriveOperation::GroveDBOperation(
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
                    .expect("total balance must be >= fee_amount");
                ops.push(DriveOperation::GroveDBOperation(
                    QualifiedGroveDbOp::insert_or_replace_op(
                        pool_path.clone(),
                        vec![SHIELDED_TOTAL_BALANCE_KEY],
                        Element::new_sum_item(new_total_balance as i64),
                    ),
                ));

                // 5. Update params with incremented checkpoint_id_counter
                let new_params = ShieldedPoolParams {
                    checkpoint_id_counter: next_checkpoint_id,
                };
                let encoded_params =
                    bincode::encode_to_vec(&new_params, bincode::config::standard())
                        .expect("expected to encode shielded pool params");
                ops.push(DriveOperation::GroveDBOperation(
                    QualifiedGroveDbOp::insert_or_replace_op(
                        pool_path,
                        vec![SHIELDED_PARAMS_KEY],
                        Element::new_item(encoded_params),
                    ),
                ));

                // Record anchor after batch is applied (finalization task)
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
                let commitments_path = shielded_credit_pool_commitments_path_vec();
                let encrypted_notes_path = shielded_credit_pool_encrypted_notes_path_vec();
                let pool_path = shielded_credit_pool_path_vec();

                // 1. Insert each nullifier (empty Item, InsertOnly to prevent double-spend)
                for nullifier in v0.nullifiers.iter() {
                    ops.push(DriveOperation::GroveDBOperation(
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

                // We start with the current checkpoint_id and increment for each commitment
                let mut next_checkpoint_id = v0.current_checkpoint_id;

                // 3. Append each note commitment (change outputs) to the commitment tree
                for cmx in v0.note_commitments.iter() {
                    next_checkpoint_id += 1;
                    ops.push(DriveOperation::GroveDBOperation(
                        QualifiedGroveDbOp::commitment_tree_append_op(
                            commitments_path.clone(),
                            vec![],
                            *cmx,
                            next_checkpoint_id,
                        ),
                    ));
                }

                // 4. Insert encrypted notes keyed by their commitment
                for (cmx, encrypted_note) in
                    v0.note_commitments.iter().zip(v0.encrypted_notes.iter())
                {
                    ops.push(DriveOperation::GroveDBOperation(
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
                    .expect("total balance must be >= unshield amount");
                ops.push(DriveOperation::GroveDBOperation(
                    QualifiedGroveDbOp::insert_or_replace_op(
                        pool_path.clone(),
                        vec![SHIELDED_TOTAL_BALANCE_KEY],
                        Element::new_sum_item(new_total_balance as i64),
                    ),
                ));

                // 6. Update params with incremented checkpoint_id_counter
                let new_params = ShieldedPoolParams {
                    checkpoint_id_counter: next_checkpoint_id,
                };
                let encoded_params =
                    bincode::encode_to_vec(&new_params, bincode::config::standard())
                        .expect("expected to encode shielded pool params");
                ops.push(DriveOperation::GroveDBOperation(
                    QualifiedGroveDbOp::insert_or_replace_op(
                        pool_path,
                        vec![SHIELDED_PARAMS_KEY],
                        Element::new_item(encoded_params),
                    ),
                ));

                // Record anchor after batch is applied (finalization task)
                ops.push(DriveOperation::FinalizeOperation(
                    DriveOperationFinalizeTask::RecordShieldedAnchor,
                ));

                Ok(ops)
            }
        }
    }
}
